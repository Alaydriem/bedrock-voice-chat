use std::sync::Arc;

use tauri::Emitter;
use tauri::State;
use tauri::async_runtime::Mutex;

use common::bedrock_protocol::RealmsApi;
use common::consts::bedrock::{
    BEDROCK_KEYRING_KEY_REFRESH_TOKEN, BEDROCK_KEYRING_KEY_XUID, BEDROCK_LISTEN_PORT,
    XBOX_CLIENT_ID,
};
use common::structs::bedrock::{
    BedrockBackendKind, BedrockConnectionInfo, BedrockStatus, HIVE_DNS_HOSTNAME, NetworkInterface,
    RealmEntry,
};
use common::traits::StreamTrait;

use crate::NetworkPacket;
use crate::analytics::AnalyticsService;
use crate::bedrock::BedrockConnectErrorChannel;
use crate::bedrock::BedrockEventEmitter;
use crate::bedrock::BedrockProxyManager;
use crate::bedrock::{
    AnnounceInjector, BedrockAuthService, BedrockKeyringService, BedrockState, JukeboxBeaconCache,
    JukeboxEjectInjector, PresenceInjector, ProtocolGatingService, ProxyDeps,
};
use crate::feature_flags::FeatureFlagService;
use crate::iap::EntitlementService;
use crate::structs::app_state::AppState;

#[tauri::command(async)]
pub(crate) async fn bedrock_start_proxy(
    app_handle: tauri::AppHandle,
    target_host: String,
    target_port: u16,
    listen_port: Option<u16>,
    network_interface: String,
    state: State<'_, Mutex<BedrockState>>,
    app_state: State<'_, Mutex<AppState>>,
    quic_producer: State<'_, Arc<flume::Sender<NetworkPacket>>>,
    flag_service: State<'_, Arc<FeatureFlagService>>,
    analytics: State<'_, Arc<AnalyticsService>>,
    beacon_cache: State<'_, Arc<JukeboxBeaconCache>>,
    eject_injector: State<'_, Arc<JukeboxEjectInjector>>,
    presence_injector: State<'_, Arc<PresenceInjector>>,
    announce_injector: State<'_, Arc<AnnounceInjector>>,
    error_channel: State<'_, Arc<BedrockConnectErrorChannel>>,
) -> Result<(), String> {
    let mut state = state.lock().await;

    if state.realms.as_ref().is_some_and(|r| !r.is_stopped()) {
        return Err("Realms session is active. Stop it before starting proxy.".to_string());
    }

    if state.proxy.as_ref().is_some_and(|p| !p.is_stopped()) {
        return Err("Proxy is already running.".to_string());
    }

    let auth_manager = state
        .auth_manager
        .as_ref()
        .ok_or_else(|| "Xbox Live authentication required. Please sign in first.".to_string())?;

    let gating = ProtocolGatingService::new_shared(
        Arc::clone(flag_service.inner()),
        Arc::clone(analytics.inner()),
    );

    let effective_listen_port = listen_port.unwrap_or(BEDROCK_LISTEN_PORT);
    let deps = ProxyDeps::new(
        Arc::clone(&state.player_state_cache),
        gating,
        Arc::clone(beacon_cache.inner()),
        Arc::clone(error_channel.inner()),
        Arc::new(BedrockEventEmitter::new(quic_producer.inner().clone())),
        Arc::clone(eject_injector.inner()),
        Arc::clone(presence_injector.inner()),
        Arc::clone(announce_injector.inner()),
    );
    let mut proxy = BedrockProxyManager::new_direct(
        target_host.clone(),
        target_port,
        effective_listen_port,
        Arc::clone(auth_manager),
        deps,
    );
    proxy.start().await.map_err(|e| e.to_string())?;

    let server_api = {
        let app = app_state.lock().await;
        if let Err(e) = state
            .start_keepalive(&app, effective_listen_port, &network_interface)
            .await
        {
            log::warn!("Transfer keepalive failed to start: {}", e);
        }
        app.api_client.clone()
    };

    let (server_transfer_relay, server_dns_enabled) = match server_api {
        Some(api) => api.resolve_bedrock_connection_hints().await,
        None => (None, false),
    };

    state.proxy = Some(proxy);
    state.proxy_target_host = Some(target_host.clone());
    state.proxy_target_port = Some(target_port);
    state.proxy_listen_port = Some(effective_listen_port);

    let info = BedrockConnectionInfo {
        local_address: "127.0.0.1".to_string(),
        lan_address: network_interface.clone(),
        port: effective_listen_port,
        backend: BedrockBackendKind::Direct,
        remote_label: format!("{}:{}", target_host, target_port),
        hive_dns_hostname: HIVE_DNS_HOSTNAME.to_string(),
        server_dns_enabled,
        server_transfer_relay,
    };
    if let Err(e) = app_handle.emit("bedrock_connection_info", &info) {
        log::warn!("Failed to emit bedrock_connection_info: {}", e);
    }

    Ok(())
}

#[tauri::command(async)]
pub(crate) async fn bedrock_stop_proxy(
    state: State<'_, Mutex<BedrockState>>,
) -> Result<(), String> {
    let mut state = state.lock().await;
    state.stop_keepalive().await;
    if let Some(ref mut proxy) = state.proxy {
        proxy.stop().await.map_err(|e| e.to_string())?;
    }
    state.proxy = None;
    state.proxy_target_host = None;
    state.proxy_target_port = None;
    state.proxy_listen_port = None;
    Ok(())
}

#[tauri::command(async)]
pub(crate) async fn bedrock_start_realms(
    app_handle: tauri::AppHandle,
    realm_id: u64,
    realm_name: String,
    network_interface: String,
    state: State<'_, Mutex<BedrockState>>,
    app_state: State<'_, Mutex<AppState>>,
    entitlement: State<'_, Arc<EntitlementService>>,
    quic_producer: State<'_, Arc<flume::Sender<NetworkPacket>>>,
    flag_service: State<'_, Arc<FeatureFlagService>>,
    analytics: State<'_, Arc<AnalyticsService>>,
    beacon_cache: State<'_, Arc<JukeboxBeaconCache>>,
    error_channel: State<'_, Arc<BedrockConnectErrorChannel>>,
    eject_injector: State<'_, Arc<JukeboxEjectInjector>>,
    presence_injector: State<'_, Arc<PresenceInjector>>,
    announce_injector: State<'_, Arc<AnnounceInjector>>,
) -> Result<(), String> {
    let gate = crate::bedrock::RealmsConnectGatingService::new(
        Arc::clone(flag_service.inner()),
        Arc::clone(analytics.inner()),
    );
    if !matches!(
        gate.evaluate(entitlement.is_entitled()).await,
        common::structs::iap::RealmsGateStatus::Allowed { .. }
    ) {
        return Err(
            "Realms Connect requires an active subscription, a free-weekend window, or a membership code."
                .to_string(),
        );
    }

    let mut state = state.lock().await;

    if state.proxy.as_ref().is_some_and(|p| !p.is_stopped()) {
        return Err("Proxy session is active. Stop it before starting realms.".to_string());
    }

    if state.realms.as_ref().is_some_and(|r| !r.is_stopped()) {
        return Err("Realms is already running.".to_string());
    }

    let realms_api = state
        .realms_api
        .as_ref()
        .ok_or_else(|| "Xbox Live authentication required. Please sign in first.".to_string())?
        .clone();

    let xbl_token = state
        .xbl_token
        .as_ref()
        .ok_or_else(|| "Xbox Live authentication required. Please sign in first.".to_string())?
        .clone();

    let user_hash = state
        .user_hash
        .as_ref()
        .ok_or_else(|| "Xbox Live authentication required. Please sign in first.".to_string())?
        .clone();

    let access_token = state
        .access_token
        .as_ref()
        .ok_or_else(|| "Xbox Live authentication required. Please sign in first.".to_string())?
        .clone();

    let gating = ProtocolGatingService::new_shared(
        Arc::clone(flag_service.inner()),
        Arc::clone(analytics.inner()),
    );

    let deps = ProxyDeps::new(
        Arc::clone(&state.player_state_cache),
        gating,
        Arc::clone(beacon_cache.inner()),
        Arc::clone(error_channel.inner()),
        Arc::new(BedrockEventEmitter::new(quic_producer.inner().clone())),
        Arc::clone(eject_injector.inner()),
        Arc::clone(presence_injector.inner()),
        Arc::clone(announce_injector.inner()),
    );
    let mut realms = BedrockProxyManager::new_realm(
        realm_id,
        BEDROCK_LISTEN_PORT,
        xbl_token,
        user_hash,
        access_token,
        realms_api,
        deps,
    );
    realms.start().await.map_err(|e| e.to_string())?;

    let server_api = {
        let app = app_state.lock().await;
        if let Err(e) = state
            .start_keepalive(&app, BEDROCK_LISTEN_PORT, &network_interface)
            .await
        {
            log::warn!("Transfer keepalive failed to start: {}", e);
        }
        app.api_client.clone()
    };

    let (server_transfer_relay, server_dns_enabled) = match server_api {
        Some(api) => api.resolve_bedrock_connection_hints().await,
        None => (None, false),
    };

    state.realms = Some(realms);
    state.active_realm_id = Some(realm_id);
    state.active_realm_name = Some(realm_name.clone());

    let info = BedrockConnectionInfo {
        local_address: "127.0.0.1".to_string(),
        lan_address: network_interface.clone(),
        port: BEDROCK_LISTEN_PORT,
        backend: BedrockBackendKind::Realm,
        remote_label: realm_name,
        hive_dns_hostname: HIVE_DNS_HOSTNAME.to_string(),
        server_dns_enabled,
        server_transfer_relay,
    };
    if let Err(e) = app_handle.emit("bedrock_connection_info", &info) {
        log::warn!("Failed to emit bedrock_connection_info: {}", e);
    }

    Ok(())
}

#[tauri::command(async)]
pub(crate) async fn bedrock_stop_realms(
    state: State<'_, Mutex<BedrockState>>,
) -> Result<(), String> {
    let mut state = state.lock().await;
    state.stop_keepalive().await;
    if let Some(ref mut realms) = state.realms {
        realms.stop().await.map_err(|e| e.to_string())?;
    }
    state.realms = None;
    state.active_realm_id = None;
    state.active_realm_name = None;
    Ok(())
}

#[tauri::command(async)]
pub(crate) async fn bedrock_xbox_login(
    state: State<'_, Mutex<BedrockState>>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let (cancel_tx, mut cancel_rx) = tokio::sync::watch::channel(false);
    {
        let mut state = state.lock().await;
        state.login_cancel_tx = Some(cancel_tx);
    }

    let app = app_handle.clone();
    let auth_future = RealmsApi::authenticate(XBOX_CLIENT_ID, move |code, url| {
        log::info!("Xbox device code: {} at {}", code, url);
        let _ = app.emit(
            "bedrock-device-code",
            serde_json::json!({
                "code": code,
                "url": url,
            }),
        );
    });

    let result = tokio::select! {
        auth_result = auth_future => {
            auth_result.map_err(|e| e.to_string())?
        }
        _ = cancel_rx.wait_for(|&v| v) => {
            return Err("Login cancelled".to_string());
        }
    };

    let (api, xbl_token, user_hash, access_token, refresh_token) = result;
    let auth = BedrockAuthService::new();
    let xuid = auth.extract_xuid(&xbl_token).await?;

    let keyring = BedrockKeyringService::new(&app_handle);
    if let Some(ref rt) = refresh_token {
        keyring.store(BEDROCK_KEYRING_KEY_REFRESH_TOKEN, rt);
    }
    keyring.store(BEDROCK_KEYRING_KEY_XUID, &xuid);

    let mut state = state.lock().await;
    state.login_cancel_tx = None;
    state.apply_auth(api, xbl_token, user_hash, access_token, refresh_token, xuid);

    Ok(())
}

#[tauri::command(async)]
pub(crate) async fn bedrock_restore_auth(
    state: State<'_, Mutex<BedrockState>>,
    app_handle: tauri::AppHandle,
) -> Result<bool, String> {
    {
        let state = state.lock().await;
        if state.auth_manager.is_some() {
            return Ok(true);
        }
    }

    let keyring = BedrockKeyringService::new(&app_handle);

    let refresh_token = match keyring.load(BEDROCK_KEYRING_KEY_REFRESH_TOKEN) {
        Some(rt) => rt,
        None => return Ok(false),
    };
    let stored_xuid = keyring.load(BEDROCK_KEYRING_KEY_XUID);

    let result = RealmsApi::authenticate_refresh(XBOX_CLIENT_ID, &refresh_token)
        .await
        .map_err(|e| {
            keyring.clear();
            e.to_string()
        })?;

    let (api, xbl_token, user_hash, access_token, new_refresh_token) = result;
    let auth = BedrockAuthService::new();
    let xuid = match stored_xuid {
        Some(x) => x,
        None => auth.extract_xuid(&xbl_token).await.map_err(|e| {
            keyring.clear();
            format!("Failed to extract XUID during auth restore: {}", e)
        })?,
    };
    keyring.store(BEDROCK_KEYRING_KEY_XUID, &xuid);

    let effective_refresh = new_refresh_token.or(Some(refresh_token));
    if let Some(ref rt) = effective_refresh {
        keyring.store(BEDROCK_KEYRING_KEY_REFRESH_TOKEN, rt);
    }

    let mut state = state.lock().await;
    state.apply_auth(
        api,
        xbl_token,
        user_hash,
        access_token,
        effective_refresh,
        xuid,
    );

    Ok(true)
}

#[tauri::command(async)]
pub(crate) async fn bedrock_force_refresh(
    state: State<'_, Mutex<BedrockState>>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let refresh_token = {
        let s = state.lock().await;
        s.refresh_token
            .clone()
            .ok_or_else(|| "No refresh token available. Sign in again.".to_string())?
    };

    let (api, xbl_token, user_hash, access_token, new_refresh_token) =
        RealmsApi::authenticate_refresh(XBOX_CLIENT_ID, &refresh_token)
            .await
            .map_err(|e| e.to_string())?;

    let stored_xuid = {
        let s = state.lock().await;
        s.xuid.clone()
    };
    let auth = BedrockAuthService::new();
    let xuid = match stored_xuid {
        Some(x) => x,
        None => auth.extract_xuid(&xbl_token).await?,
    };

    let keyring = BedrockKeyringService::new(&app_handle);
    let effective_refresh = new_refresh_token.or(Some(refresh_token));
    if let Some(ref rt) = effective_refresh {
        keyring.store(BEDROCK_KEYRING_KEY_REFRESH_TOKEN, rt);
    }
    keyring.store(BEDROCK_KEYRING_KEY_XUID, &xuid);

    let mut state = state.lock().await;
    state.apply_auth(
        api,
        xbl_token,
        user_hash,
        access_token,
        effective_refresh,
        xuid,
    );
    Ok(())
}

#[tauri::command(async)]
pub(crate) async fn bedrock_cancel_xbox_login(
    state: State<'_, Mutex<BedrockState>>,
) -> Result<(), String> {
    let mut state = state.lock().await;
    if let Some(tx) = state.login_cancel_tx.take() {
        let _ = tx.send(true);
    }
    Ok(())
}

#[tauri::command(async)]
pub(crate) async fn bedrock_xbox_logout(
    state: State<'_, Mutex<BedrockState>>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let mut state = state.lock().await;

    if state.proxy.as_ref().is_some_and(|p| !p.is_stopped()) {
        return Err("Stop the proxy before signing out.".to_string());
    }
    if state.realms.as_ref().is_some_and(|r| !r.is_stopped()) {
        return Err("Stop the realms session before signing out.".to_string());
    }

    state.auth_manager = None;
    state.realms_api = None;
    state.xbl_token = None;
    state.user_hash = None;
    state.access_token = None;
    state.refresh_token = None;
    state.xuid = None;

    BedrockKeyringService::new(&app_handle).clear();
    Ok(())
}

#[tauri::command(async)]
pub(crate) async fn bedrock_list_interfaces() -> Result<Vec<NetworkInterface>, String> {
    let mut interfaces: Vec<NetworkInterface> = if_addrs::get_if_addrs()
        .map_err(|e| e.to_string())?
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
    // Bedrock clients (especially on mobile) reach BVC over IPv4 in practice,
    // so surface IPv4 entries first — both for the default selection and for
    // dropdown ordering.
    interfaces.sort_by_key(|iface| !iface.is_ipv4);
    Ok(interfaces)
}

#[tauri::command(async)]
pub(crate) async fn bedrock_list_realms(
    state: State<'_, Mutex<BedrockState>>,
) -> Result<Vec<RealmEntry>, String> {
    let api = {
        let state = state.lock().await;
        state
            .realms_api
            .as_ref()
            .ok_or_else(|| "Xbox Live authentication required. Please sign in first.".to_string())?
            .clone()
    };

    let worlds = api.list_worlds().await.map_err(|e| e.to_string())?;
    let entries = worlds
        .into_iter()
        .map(|r| RealmEntry {
            id: r.id,
            name: r.name,
            motd: r.motd,
            state: r.state,
            owner_uuid: r.owner_uuid,
        })
        .collect();
    Ok(entries)
}

#[tauri::command(async)]
pub(crate) async fn bedrock_get_position(
    state: State<'_, Mutex<BedrockState>>,
) -> Result<Option<String>, String> {
    let state = state.lock().await;
    let player = state.player_state_cache.get_local_player();
    match player {
        Some(p) => Ok(Some(format!("{:?}", p))),
        None => Ok(None),
    }
}

#[tauri::command(async)]
pub(crate) async fn bedrock_get_status(
    state: State<'_, Mutex<BedrockState>>,
) -> Result<BedrockStatus, String> {
    let state = state.lock().await;
    Ok(BedrockStatus {
        proxy_running: state.proxy.as_ref().is_some_and(|p| !p.is_stopped()),
        realms_running: state.realms.as_ref().is_some_and(|r| !r.is_stopped()),
        xbox_authenticated: state.auth_manager.is_some(),
        proxy_target_host: state.proxy_target_host.clone(),
        proxy_target_port: state.proxy_target_port,
        proxy_listen_port: state.proxy_listen_port,
        active_realm_id: state.active_realm_id,
        active_realm_name: state.active_realm_name.clone(),
    })
}

#[tauri::command(async)]
pub(crate) async fn bedrock_realms_gate(
    entitlement: State<'_, Arc<EntitlementService>>,
    flag_service: State<'_, Arc<FeatureFlagService>>,
    analytics: State<'_, Arc<AnalyticsService>>,
) -> Result<common::structs::iap::RealmsGateStatus, String> {
    let gate = crate::bedrock::RealmsConnectGatingService::new(
        Arc::clone(flag_service.inner()),
        Arc::clone(analytics.inner()),
    );
    Ok(gate.evaluate(entitlement.is_entitled()).await)
}
