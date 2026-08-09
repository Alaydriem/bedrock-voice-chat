pub mod proxy_request;
pub mod realm_request;

pub use proxy_request::ProxyConnectRequest;
pub use realm_request::RealmConnectRequest;

use std::sync::Arc;

use tauri::Emitter;
use tauri::Manager;
use tauri::async_runtime::Mutex;

use common::consts::bedrock::BEDROCK_LISTEN_PORT;
use common::structs::bedrock::{
    BedrockBackendKind, BedrockConnectionInfo, HIVE_DNS_HOSTNAME, NetworkInterface,
};
use common::traits::StreamTrait;

use crate::NetworkPacket;
use crate::analytics::AnalyticsService;
use crate::bedrock::{
    AdvertisedVersionResolver, AnnounceInjector, BedrockChatChannel, BedrockConnectErrorChannel,
    BedrockEventEmitter, BedrockProxyManager, BedrockState, ChatInjector, JukeboxBeaconCache,
    JukeboxEjectInjector, PresenceInjector, ProtocolGatingService, ProxyDeps,
};
use crate::control::ControlActionSender;
use crate::feature_flags::FeatureFlagService;
use crate::feature_flags::flags::bedrock::RealmsConnectEnabled;
use crate::structs::app_state::AppState;

/// Starts a Bedrock session against a direct backend or a Realm.
///
/// The Tauri commands and the WebSocket control surface both reach a world through this, so
/// a scripted connect and a click cannot drift apart in what they wire up, which port they
/// listen on, or what they report afterwards.
///
/// Dependencies are resolved from managed state rather than taken as arguments: there are a
/// dozen of them, they are all `.manage()`d already, and threading them through every caller
/// buys nothing.
pub struct BedrockConnector {
    app_handle: tauri::AppHandle,
}

impl BedrockConnector {
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        Self { app_handle }
    }

    pub fn new_shared(app_handle: tauri::AppHandle) -> Arc<Self> {
        Arc::new(Self::new(app_handle))
    }

    /// Every non-loopback interface, IPv4 first.
    ///
    /// Bedrock clients (especially on mobile) reach BVC over IPv4 in practice, so IPv4 entries
    /// lead — both for the default selection and for dropdown ordering.
    pub fn interfaces() -> Result<Vec<NetworkInterface>, anyhow::Error> {
        let mut interfaces: Vec<NetworkInterface> = if_addrs::get_if_addrs()?
            .into_iter()
            .filter(|iface| !iface.is_loopback())
            .map(|iface| {
                let ip = iface.ip();
                NetworkInterface {
                    name: iface.name.clone(),
                    ip: ip.to_string(),
                    is_ipv4: ip.is_ipv4(),
                }
            })
            .collect();
        interfaces.sort_by_key(|iface| !iface.is_ipv4);
        Ok(interfaces)
    }

    /// The interface a caller that expressed no preference gets.
    ///
    /// The same first-IPv4 choice the settings pane defaults to, so a scripted connect and a
    /// click land on the same address.
    pub fn default_interface() -> Option<String> {
        Self::interfaces()
            .ok()?
            .into_iter()
            .next()
            .map(|iface| iface.ip)
    }

    pub async fn start_proxy(&self, request: ProxyConnectRequest) -> Result<(), anyhow::Error> {
        let state = self.app_handle.state::<Mutex<BedrockState>>();
        let mut state = state.lock().await;

        if state.realms.as_ref().is_some_and(|r| !r.is_stopped()) {
            anyhow::bail!("Realms session is active. Stop it before starting proxy.");
        }

        if state.proxy.as_ref().is_some_and(|p| !p.is_stopped()) {
            anyhow::bail!("Proxy is already running.");
        }

        let auth_manager = state
            .auth_manager
            .as_ref()
            .ok_or_else(|| {
                anyhow::anyhow!("Xbox Live authentication required. Please sign in first.")
            })?
            .clone();

        let network_interface = request
            .network_interface
            .or_else(Self::default_interface)
            .unwrap_or_default();

        let effective_listen_port = request.listen_port.unwrap_or(BEDROCK_LISTEN_PORT);
        let deps = self.proxy_deps(&state);
        let advertised_version = AdvertisedVersionResolver::proxy(request.advertised_protocol);
        let mut proxy = BedrockProxyManager::new_direct(
            request.target_host.clone(),
            request.target_port,
            effective_listen_port,
            auth_manager,
            advertised_version,
            deps,
        );
        proxy.start().await?;

        let (server_transfer_relay, server_dns_enabled) = self
            .start_keepalive(&mut state, effective_listen_port, &network_interface)
            .await;

        state.proxy = Some(proxy);
        state.proxy_target_host = Some(request.target_host.clone());
        state.proxy_target_port = Some(request.target_port);
        state.proxy_listen_port = Some(effective_listen_port);
        state.proxy_started_at = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        );

        self.emit_connection_info(BedrockConnectionInfo {
            local_address: "127.0.0.1".to_string(),
            lan_address: network_interface,
            port: effective_listen_port,
            backend: BedrockBackendKind::Direct,
            remote_label: format!("{}:{}", request.target_host, request.target_port),
            hive_dns_hostname: HIVE_DNS_HOSTNAME.to_string(),
            server_dns_enabled,
            server_transfer_relay,
        });

        Ok(())
    }

    pub async fn start_realm(&self, request: RealmConnectRequest) -> Result<(), anyhow::Error> {
        let flag_service = self.app_handle.state::<Arc<FeatureFlagService>>();

        // Scope is deliberate: the switch is read once, here, so it stops new
        // sessions from starting. A session already running is not torn down when
        // the flag flips off — it ends when the player disconnects. Cutting live
        // sessions would need a watcher on the flag generation, not a check here.
        if !flag_service.get(RealmsConnectEnabled).await {
            anyhow::bail!("Realms Connect is currently unavailable.");
        }

        let state = self.app_handle.state::<Mutex<BedrockState>>();
        let mut state = state.lock().await;

        if state.proxy.as_ref().is_some_and(|p| !p.is_stopped()) {
            anyhow::bail!("Proxy session is active. Stop it before starting realms.");
        }

        if state.realms.as_ref().is_some_and(|r| !r.is_stopped()) {
            anyhow::bail!("Realms is already running.");
        }

        let realms_api = Self::require_auth(state.realms_api.clone())?;
        let xbl_token = Self::require_auth(state.xbl_token.clone())?;
        let user_hash = Self::require_auth(state.user_hash.clone())?;
        let access_token = Self::require_auth(state.access_token.clone())?;

        let network_interface = request
            .network_interface
            .or_else(Self::default_interface)
            .unwrap_or_default();

        let deps = self.proxy_deps(&state);
        let advertised_version = AdvertisedVersionResolver::realms(flag_service.inner()).await;
        let mut realms = BedrockProxyManager::new_realm(
            request.realm_id,
            BEDROCK_LISTEN_PORT,
            xbl_token,
            user_hash,
            access_token,
            realms_api,
            advertised_version,
            deps,
        );
        realms.start().await?;

        let (server_transfer_relay, server_dns_enabled) = self
            .start_keepalive(&mut state, BEDROCK_LISTEN_PORT, &network_interface)
            .await;

        state.realms = Some(realms);
        state.active_realm_id = Some(request.realm_id);
        state.active_realm_name = Some(request.realm_name.clone());

        self.emit_connection_info(BedrockConnectionInfo {
            local_address: "127.0.0.1".to_string(),
            lan_address: network_interface,
            port: BEDROCK_LISTEN_PORT,
            backend: BedrockBackendKind::Realm,
            remote_label: request.realm_name,
            hive_dns_hostname: HIVE_DNS_HOSTNAME.to_string(),
            server_dns_enabled,
            server_transfer_relay,
        });

        Ok(())
    }

    fn require_auth<T>(value: Option<T>) -> Result<T, anyhow::Error> {
        value.ok_or_else(|| {
            anyhow::anyhow!("Xbox Live authentication required. Please sign in first.")
        })
    }

    fn proxy_deps(&self, state: &BedrockState) -> ProxyDeps {
        let gating = ProtocolGatingService::new_shared(
            Arc::clone(self.app_handle.state::<Arc<FeatureFlagService>>().inner()),
            Arc::clone(self.app_handle.state::<Arc<AnalyticsService>>().inner()),
        );

        ProxyDeps::new(
            Arc::clone(&state.player_state_cache),
            gating,
            Arc::clone(self.app_handle.state::<Arc<JukeboxBeaconCache>>().inner()),
            Arc::clone(
                self.app_handle
                    .state::<Arc<BedrockConnectErrorChannel>>()
                    .inner(),
            ),
            Arc::clone(self.app_handle.state::<Arc<BedrockChatChannel>>().inner()),
            Arc::clone(self.app_handle.state::<Arc<ChatInjector>>().inner()),
            Arc::new(BedrockEventEmitter::new(
                self.app_handle
                    .state::<Arc<flume::Sender<NetworkPacket>>>()
                    .inner()
                    .clone(),
            )),
            Arc::clone(self.app_handle.state::<Arc<JukeboxEjectInjector>>().inner()),
            Arc::clone(self.app_handle.state::<Arc<PresenceInjector>>().inner()),
            Arc::clone(self.app_handle.state::<Arc<AnnounceInjector>>().inner()),
            self.app_handle.state::<ControlActionSender>().inner().clone(),
            self.app_handle
                .state::<Arc<crate::bedrock::QueryStateInjector>>()
                .inner()
                .clone(),
            self.app_handle
                .state::<crate::control::ControlStateBus>()
                .inner()
                .clone(),
        )
    }

    // Starts the transfer keepalive and resolves the hints the connection card shows. A
    // keepalive that fails to start is logged rather than fatal: the session itself is up, and
    // the keepalive only smooths reconnects.
    async fn start_keepalive(
        &self,
        state: &mut BedrockState,
        listen_port: u16,
        network_interface: &str,
    ) -> (Option<String>, bool) {
        let app_state = self.app_handle.state::<Mutex<AppState>>();
        let server_api = {
            let app = app_state.lock().await;
            if let Err(e) = state
                .start_keepalive(&app, listen_port, network_interface)
                .await
            {
                log::warn!("Transfer keepalive failed to start: {}", e);
            }
            app.api_client.clone()
        };

        match server_api {
            Some(api) => api.resolve_bedrock_connection_hints().await,
            None => (None, false),
        }
    }

    fn emit_connection_info(&self, info: BedrockConnectionInfo) {
        if let Err(e) = self.app_handle.emit("bedrock_connection_info", &info) {
            log::warn!("Failed to emit bedrock_connection_info: {}", e);
        }
    }
}
