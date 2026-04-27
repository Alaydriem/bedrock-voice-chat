use std::sync::Arc;

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use tauri::State;
use tauri::async_runtime::Mutex;
use tauri_plugin_keyring::{CredentialType, CredentialValue, KeyringExt};

use bedrock_protocol::{AuthManager, CachedToken, RealmsApi};
use bedrock_protocol::auth::xbox::XboxLive;
use common::structs::bedrock::BedrockStatus;
use common::structs::bedrock::NetworkInterface;
use common::structs::bedrock::RealmEntry;
use common::traits::StreamTrait;

use crate::bedrock::BedrockState;
use crate::bedrock::iap::BedrockEntitlementCheck;
use crate::bedrock::keepalive::TransferKeepalive;
use crate::bedrock::proxy::ProxyConnectManager;
use crate::bedrock::realms::RealmsConnectManager;
use crate::structs::app_state::AppState;

use common::consts::bedrock::{BEDROCK_LISTEN_PORT, XBOX_CLIENT_ID};
use tauri::Emitter;

const BEDROCK_KEYRING_NS: &str = "bedrock-xbox";
const KEY_REFRESH_TOKEN: &str = "refresh_token";
const KEY_XUID: &str = "xuid";

fn keyring_key(key: &str) -> String {
    BASE64.encode(format!("{}/{}", BEDROCK_KEYRING_NS, key))
}

fn store_credential(app: &tauri::AppHandle, key: &str, value: &str) {
    let encoded = keyring_key(key);
    let _ = app.keyring().set(
        &encoded,
        CredentialType::Password,
        CredentialValue::Password(value.to_string()),
    );
}

fn load_credential(app: &tauri::AppHandle, key: &str) -> Option<String> {
    let encoded = keyring_key(key);
    match app.keyring().get(&encoded, CredentialType::Password) {
        Ok(CredentialValue::Password(v)) => Some(v),
        _ => None,
    }
}

fn clear_credentials(app: &tauri::AppHandle) {
    for key in [KEY_REFRESH_TOKEN, KEY_XUID] {
        let encoded = keyring_key(key);
        let _ = app.keyring().delete(&encoded, CredentialType::Password);
    }
}

fn build_auth_manager(refresh_token: Option<&str>, xuid: &str) -> Arc<AuthManager> {
    let cache = moka::future::Cache::builder()
        .time_to_live(std::time::Duration::from_secs(86400))
        .max_capacity(100)
        .build();

    let mgr = Arc::new(AuthManager::new(XBOX_CLIENT_ID, cache));

    if let Some(rt) = refresh_token {
        let cache = mgr.cache().clone();
        let rt = rt.to_string();
        let xuid = xuid.to_string();
        tauri::async_runtime::spawn(async move {
            cache.insert(xuid, CachedToken { refresh_token: rt }).await;
        });
    }

    mgr
}

async fn extract_xuid(xbl_token: &str) -> Result<String, String> {
    let xsts = XboxLive::authenticate_xsts(xbl_token, "http://xboxlive.com")
        .await
        .map_err(|e| format!("XSTS authentication failed: {}", e))?;
    xsts.display_claims
        .xui
        .first()
        .and_then(|c| c.xid.clone())
        .ok_or_else(|| "XUID not present in XSTS response".to_string())
}

fn apply_auth_to_state(
    state: &mut BedrockState,
    api: RealmsApi,
    xbl_token: String,
    user_hash: String,
    access_token: String,
    refresh_token: Option<String>,
    xuid: String,
) {
    let auth_manager = build_auth_manager(
        refresh_token.as_deref(),
        &xuid,
    );

    state.auth_manager = Some(auth_manager);
    state.realms_api = Some(api);
    state.xbl_token = Some(xbl_token);
    state.user_hash = Some(user_hash);
    state.access_token = Some(access_token);
    state.refresh_token = refresh_token;
    state.xuid = Some(xuid);
}

async fn start_keepalive(
    bedrock_state: &mut BedrockState,
    app_state: &AppState,
    listen_port: u16,
    network_interface: &str,
) -> Result<(), String> {
    let xuid = bedrock_state.xuid.as_ref()
        .ok_or_else(|| "XUID required for transfer keepalive".to_string())?
        .clone();

    let api = app_state.get_api_client()
        .map_err(|e| format!("BVC server connection required: {}", e))?;

    let server_url = api.endpoint().to_string();
    let client = api.get_reqwest_client().await;

    let mut keepalive = TransferKeepalive::new(
        server_url,
        xuid,
        network_interface.to_string(),
        listen_port,
        client,
    );
    keepalive.start().await.map_err(|e| e.to_string())?;
    bedrock_state.keepalive = Some(keepalive);
    Ok(())
}

async fn stop_keepalive(bedrock_state: &mut BedrockState) {
    if let Some(ref mut keepalive) = bedrock_state.keepalive {
        let _ = keepalive.stop().await;
    }
    bedrock_state.keepalive = None;
}

#[tauri::command(async)]
pub(crate) async fn bedrock_start_proxy(
    target_host: String,
    target_port: u16,
    listen_port: Option<u16>,
    network_interface: String,
    state: State<'_, Mutex<BedrockState>>,
    app_state: State<'_, Mutex<AppState>>,
    entitlement: State<'_, BedrockEntitlementCheck>,
) -> Result<(), String> {
    entitlement.require_entitlement()?;

    let mut state = state.lock().await;

    if state.realms.as_ref().is_some_and(|r| !r.is_stopped()) {
        return Err("Realms session is active. Stop it before starting proxy.".to_string());
    }

    if state.proxy.as_ref().is_some_and(|p| !p.is_stopped()) {
        return Err("Proxy is already running.".to_string());
    }

    let auth_manager = state.auth_manager.as_ref()
        .ok_or_else(|| "Xbox Live authentication required. Please sign in first.".to_string())?;

    let effective_listen_port = listen_port.unwrap_or(BEDROCK_LISTEN_PORT);
    let mut proxy = ProxyConnectManager::new(
        target_host.clone(),
        target_port,
        effective_listen_port,
        Arc::clone(auth_manager),
        Arc::clone(&state.position_cache),
    );
    proxy.start().await.map_err(|e| e.to_string())?;

    let app = app_state.lock().await;
    if let Err(e) = start_keepalive(&mut state, &app, effective_listen_port, &network_interface).await {
        log::warn!("Transfer keepalive failed to start: {}", e);
    }

    state.proxy = Some(proxy);
    state.proxy_target_host = Some(target_host);
    state.proxy_target_port = Some(target_port);
    state.proxy_listen_port = Some(effective_listen_port);
    Ok(())
}

#[tauri::command(async)]
pub(crate) async fn bedrock_stop_proxy(
    state: State<'_, Mutex<BedrockState>>,
) -> Result<(), String> {
    let mut state = state.lock().await;
    stop_keepalive(&mut state).await;
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
    realm_id: u64,
    realm_name: String,
    network_interface: String,
    state: State<'_, Mutex<BedrockState>>,
    app_state: State<'_, Mutex<AppState>>,
    entitlement: State<'_, BedrockEntitlementCheck>,
) -> Result<(), String> {
    entitlement.require_entitlement()?;

    let mut state = state.lock().await;

    if state.proxy.as_ref().is_some_and(|p| !p.is_stopped()) {
        return Err("Proxy session is active. Stop it before starting realms.".to_string());
    }

    if state.realms.as_ref().is_some_and(|r| !r.is_stopped()) {
        return Err("Realms is already running.".to_string());
    }

    let realms_api = state.realms_api.as_ref()
        .ok_or_else(|| "Xbox Live authentication required. Please sign in first.".to_string())?
        .clone();

    let xbl_token = state.xbl_token.as_ref()
        .ok_or_else(|| "Xbox Live authentication required. Please sign in first.".to_string())?
        .clone();

    let user_hash = state.user_hash.as_ref()
        .ok_or_else(|| "Xbox Live authentication required. Please sign in first.".to_string())?
        .clone();

    let access_token = state.access_token.as_ref()
        .ok_or_else(|| "Xbox Live authentication required. Please sign in first.".to_string())?
        .clone();

    let mut realms = RealmsConnectManager::new(
        realm_id,
        BEDROCK_LISTEN_PORT,
        xbl_token,
        user_hash,
        access_token,
        realms_api,
        Arc::clone(&state.position_cache),
    );
    realms.start().await.map_err(|e| e.to_string())?;

    let app = app_state.lock().await;
    if let Err(e) = start_keepalive(&mut state, &app, BEDROCK_LISTEN_PORT, &network_interface).await {
        log::warn!("Transfer keepalive failed to start: {}", e);
    }

    state.realms = Some(realms);
    state.active_realm_id = Some(realm_id);
    state.active_realm_name = Some(realm_name);
    Ok(())
}

#[tauri::command(async)]
pub(crate) async fn bedrock_stop_realms(
    state: State<'_, Mutex<BedrockState>>,
) -> Result<(), String> {
    let mut state = state.lock().await;
    stop_keepalive(&mut state).await;
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
        let _ = app.emit("bedrock-device-code", serde_json::json!({
            "code": code,
            "url": url,
        }));
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
    let xuid = extract_xuid(&xbl_token).await?;

    if let Some(ref rt) = refresh_token {
        store_credential(&app_handle, KEY_REFRESH_TOKEN, rt);
    }
    store_credential(&app_handle, KEY_XUID, &xuid);

    let mut state = state.lock().await;
    state.login_cancel_tx = None;
    apply_auth_to_state(
        &mut state,
        api,
        xbl_token,
        user_hash,
        access_token,
        refresh_token,
        xuid,
    );

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

    let refresh_token = match load_credential(&app_handle, KEY_REFRESH_TOKEN) {
        Some(rt) => rt,
        None => return Ok(false),
    };
    let stored_xuid = load_credential(&app_handle, KEY_XUID);

    let result = RealmsApi::authenticate_refresh(XBOX_CLIENT_ID, &refresh_token)
        .await
        .map_err(|e| {
            clear_credentials(&app_handle);
            e.to_string()
        })?;

    let (api, xbl_token, user_hash, access_token, new_refresh_token) = result;
    let xuid = match stored_xuid {
        Some(x) => x,
        None => {
            extract_xuid(&xbl_token).await.map_err(|e| {
                clear_credentials(&app_handle);
                format!("Failed to extract XUID during auth restore: {}", e)
            })?
        }
    };
    store_credential(&app_handle, KEY_XUID, &xuid);

    let effective_refresh = new_refresh_token.or(Some(refresh_token));
    if let Some(ref rt) = effective_refresh {
        store_credential(&app_handle, KEY_REFRESH_TOKEN, rt);
    }

    let mut state = state.lock().await;
    apply_auth_to_state(
        &mut state,
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

    clear_credentials(&app_handle);
    Ok(())
}

#[tauri::command(async)]
pub(crate) async fn bedrock_list_interfaces() -> Result<Vec<NetworkInterface>, String> {
    let interfaces = if_addrs::get_if_addrs()
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter(|iface| !iface.is_loopback())
        .map(|iface| NetworkInterface {
            name: iface.name.clone(),
            ip: iface.ip().to_string(),
        })
        .collect();
    Ok(interfaces)
}

#[tauri::command(async)]
pub(crate) async fn bedrock_list_realms(
    state: State<'_, Mutex<BedrockState>>,
) -> Result<Vec<RealmEntry>, String> {
    let api = {
        let state = state.lock().await;
        state.realms_api.as_ref()
            .ok_or_else(|| "Xbox Live authentication required. Please sign in first.".to_string())?
            .clone()
    };

    let worlds = api.list_worlds().await.map_err(|e| e.to_string())?;
    let entries = worlds.into_iter().map(|r| RealmEntry {
        id: r.id,
        name: r.name,
        motd: r.motd,
        state: r.state,
        owner_uuid: r.owner_uuid,
    }).collect();
    Ok(entries)
}

#[tauri::command(async)]
pub(crate) async fn bedrock_get_position(
    state: State<'_, Mutex<BedrockState>>,
) -> Result<Option<String>, String> {
    let state = state.lock().await;
    let player = state.position_cache.get_local_player();
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
pub(crate) async fn bedrock_check_entitlement(
    entitlement: State<'_, BedrockEntitlementCheck>,
) -> Result<bool, String> {
    Ok(entitlement.is_entitled())
}
