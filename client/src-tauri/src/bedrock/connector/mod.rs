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
    BedrockBackendKind, BedrockConnectionInfo, BedrockRenewal, NetworkInterface,
};
use common::traits::StreamTrait;

use crate::NetworkPacket;
use crate::analytics::AnalyticsService;
use websocket_types::{ActiveConnection, ConnectTargetId, ConnectTargetKind, ConnectTargetSource};

use crate::bedrock::{
    AddonModeResolver, AdvertisedVersionResolver, BedrockAuthService,
    BedrockChatChannel,
    SessionName,
    BedrockConnectErrorChannel,
    BedrockEventEmitter, BedrockProxyManager, BedrockState, BedrockTargetService, ChatInjector,
    JukeboxBeaconCache, JukeboxEjectInjector, ProtocolGatingService, ProxyDeps,
};
use crate::control::ControlActionSender;
use crate::feature_flags::FeatureFlagService;
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

    /// The operator-curated list, read through the same cached `/api/config` the
    /// connection hints already use.
    ///
    /// A failure is reported as an empty list rather than an error: it resolves the
    /// session to no-net, which is the conservative direction — a world wrongly
    /// treated as net loses voice features silently.
    async fn advertised_servers(&self) -> Vec<common::response::ApiConfigBedrockServer> {
        let app_state = self.app_handle.state::<Mutex<AppState>>();
        let api = {
            let app = app_state.lock().await;
            app.api_client.clone()
        };

        let Some(api) = api else {
            log::warn!("No API client; resolving the addon transport as no-net");
            return vec![];
        };

        match api.get_config().await {
            Ok(config) => config.bedrock.servers,
            Err(e) => {
                log::warn!(
                    "Could not read the advertised server list; resolving as no-net: {}",
                    e
                );
                vec![]
            }
        }
    }

    /// The signed-in gamertag, for the advert's second line.
    ///
    /// Absent rather than fatal: a session that cannot name its owner still
    /// connects, it just advertises branding alone.
    async fn owner_gamertag(&self) -> Option<String> {
        let server = {
            let app_state = self.app_handle.state::<Mutex<AppState>>();
            let app = app_state.lock().await;
            app.current_server.clone()
        }?;

        // `try_state`, not `state`: the keyring is managed in the app's own run path,
        // which the e2e binary never executes. `state` would panic there, taking the
        // whole session down to decorate an advert.
        let keyring = self
            .app_handle
            .try_state::<Mutex<crate::keyring::KeyringService>>()?;
        let mut kr = keyring.lock().await;
        kr.get_credential(&server, "gamertag").ok()
    }

    pub async fn start_proxy(&self, request: ProxyConnectRequest) -> Result<(), anyhow::Error> {
        // Resolved before the state lock: this reads the cached config over the
        // network client, which must not happen with the BedrockState guard held.
        let advertised = self.advertised_servers().await;
        let addon_mode = AddonModeResolver::proxy(
            request.addon_mode,
            &advertised,
            &request.target_host,
            request.target_port,
        );

        // Both resolved before the state lock. `name_proxy_session` reaches
        // `load_proxies`, which takes that same lock, and the advert needs the name
        // at construction rather than after the session is up.
        let named = self
            .name_proxy_session(&request.target_host, request.target_port)
            .await;
        let world = SessionName::world(
            named.as_ref().map(|c| c.name.as_str()),
            &request.target_host,
        );
        let owner = SessionName::owner(self.owner_gamertag().await.as_deref());

        let state = self.app_handle.state::<Mutex<BedrockState>>();

        let info = {
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
                .ok_or_else(|| anyhow::anyhow!(crate::bedrock::XBOX_AUTH_REQUIRED))?
                .clone();

            let network_interface = request
                .network_interface
                .clone()
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
                addon_mode,
                world,
                owner,
                deps,
            );
            proxy.start().await?;

            let server_transfer_relay = self
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

            BedrockConnectionInfo {
                local_address: "127.0.0.1".to_string(),
                lan_address: network_interface,
                port: effective_listen_port,
                backend: BedrockBackendKind::Direct,
                remote_label: format!("{}:{}", request.target_host, request.target_port),
                server_transfer_relay,
            }
        };

        self.emit_connection_info(info);

        state.lock().await.active_connection = named;

        self.broadcast_state().await;

        Ok(())
    }

    pub async fn start_realm(&self, request: RealmConnectRequest) -> Result<(), anyhow::Error> {
        // Resolved before the lock for the same reason the proxy path does it: the
        // keyring read must not happen with the BedrockState guard held.
        let realm_owner = SessionName::owner(self.owner_gamertag().await.as_deref());

        let flag_service = self.app_handle.state::<Arc<FeatureFlagService>>();

        let state = self.app_handle.state::<Mutex<BedrockState>>();

        // The realm backend carries a fixed AuthInfo minted here, with no refresh path of its
        // own, so this is the only chance to notice the credential has expired. Outside the
        // guard below: `renew` locks this same state.
        match BedrockAuthService::new()
            .renew(state.inner(), &self.app_handle)
            .await
        {
            BedrockRenewal::ReauthRequired => {
                anyhow::bail!("Xbox re-authentication required. Please sign in again.");
            }
            // The tokens already held may still work, and refusing on a network blip would be
            // worse than trying with them.
            BedrockRenewal::Unavailable { message } => {
                log::warn!("Could not renew before realm connect: {}", message);
            }
            BedrockRenewal::Renewed => {}
        }

        let info = {
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
                .clone()
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
                request.realm_name.clone(),
                realm_owner,
                deps,
            );
            realms.start().await?;

            let server_transfer_relay = self
                .start_keepalive(&mut state, BEDROCK_LISTEN_PORT, &network_interface)
                .await;

            state.realms = Some(realms);
            state.active_realm_id = Some(request.realm_id);
            state.active_realm_name = Some(request.realm_name.clone());
            state.active_connection = Some(ActiveConnection::new(
                ConnectTargetId::mint(ConnectTargetSource::Realm, &request.realm_id.to_string()),
                request.realm_name.clone(),
                ConnectTargetKind::Realm,
            ));

            BedrockConnectionInfo {
                local_address: "127.0.0.1".to_string(),
                lan_address: network_interface,
                port: BEDROCK_LISTEN_PORT,
                backend: BedrockBackendKind::Realm,
                remote_label: request.realm_name.clone(),
                server_transfer_relay,
            }
        };

        self.emit_connection_info(info);
        self.broadcast_state().await;

        Ok(())
    }

    /// Stops the proxy session, reporting the world it was connected to.
    pub async fn stop_proxy(&self) -> Result<Option<ActiveConnection>, anyhow::Error> {
        let stopped = {
            let state = self.app_handle.state::<Mutex<BedrockState>>();
            let mut state = state.lock().await;
            state.stop_keepalive().await;
            if let Some(ref mut proxy) = state.proxy {
                proxy.stop().await?;
            }
            state.proxy = None;
            state.proxy_target_host = None;
            state.proxy_target_port = None;
            state.proxy_listen_port = None;
            state.proxy_started_at = None;
            state.active_connection.take()
        };

        self.broadcast_state().await;
        Ok(stopped)
    }

    /// Stops the realms session, reporting the world it was connected to.
    pub async fn stop_realm(&self) -> Result<Option<ActiveConnection>, anyhow::Error> {
        let stopped = {
            let state = self.app_handle.state::<Mutex<BedrockState>>();
            let mut state = state.lock().await;
            state.stop_keepalive().await;
            if let Some(ref mut realms) = state.realms {
                realms.stop().await?;
            }
            state.realms = None;
            state.active_realm_id = None;
            state.active_realm_name = None;
            state.active_connection.take()
        };

        self.broadcast_state().await;
        Ok(stopped)
    }

    /// Stops whichever session is live.
    ///
    /// Only one can be: `start_proxy` and `start_realm` each refuse while the other is
    /// running. Idempotent, and reports what it stopped, so a controller that lost track of
    /// the client can send this without first asking what is running.
    pub async fn disconnect(&self) -> Result<Option<ActiveConnection>, anyhow::Error> {
        let (proxy_running, realms_running) = {
            let state = self.app_handle.state::<Mutex<BedrockState>>();
            let state = state.lock().await;
            (
                state.proxy.as_ref().is_some_and(|p| !p.is_stopped()),
                state.realms.as_ref().is_some_and(|r| !r.is_stopped()),
            )
        };

        if proxy_running {
            return self.stop_proxy().await;
        }
        if realms_running {
            return self.stop_realm().await;
        }
        Ok(None)
    }

    /// The saved or advertised entry a proxy session was started against.
    ///
    /// Resolved from the same list a `targets` listing is built from, so the two cannot
    /// disagree about what a world is called. A miss is reported as absent rather than
    /// invented, which leaves a controller with a live session it cannot correlate — better
    /// than one it correlates wrongly.
    ///
    /// Takes the `BedrockState` lock through `load_proxies`, so it must be called with that
    /// guard closed.
    async fn name_proxy_session(&self, host: &str, port: u16) -> Option<ActiveConnection> {
        let service = match BedrockTargetService::load_proxies(&self.app_handle).await {
            Ok(service) => service,
            Err(e) => {
                log::warn!("Could not name the proxy session: {}", e);
                return None;
            }
        };

        match service.resolve_by_address(host, port) {
            Some(target) => Some(target.to_active()),
            None => {
                log::warn!("No saved or advertised entry names {}:{}", host, port);
                None
            }
        }
    }

    /// Pushes the rebuilt state frame to every WebSocket client.
    ///
    /// Called after a session starts or stops, from whichever surface did it. Without this a
    /// user pressing Stop in the app leaves every controller drawing a connected world.
    ///
    /// Must be called with the `BedrockState` guard closed: building the frame reads the
    /// active connection from behind that same lock.
    async fn broadcast_state(&self) {
        crate::audio::AudioActionsManager::new(self.app_handle.clone())
            .broadcast_state()
            .await;
    }

    fn require_auth<T>(value: Option<T>) -> Result<T, anyhow::Error> {
        value.ok_or_else(|| anyhow::anyhow!(crate::bedrock::XBOX_AUTH_REQUIRED))
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
    ) -> Option<String> {
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
            None => None,
        }
    }

    fn emit_connection_info(&self, info: BedrockConnectionInfo) {
        if let Err(e) = self.app_handle.emit("bedrock_connection_info", &info) {
            log::warn!("Failed to emit bedrock_connection_info: {}", e);
        }
    }
}
