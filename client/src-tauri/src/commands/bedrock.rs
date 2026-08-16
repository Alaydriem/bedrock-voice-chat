use std::sync::Arc;

use tauri::Emitter;
use tauri::State;
use tauri::async_runtime::Mutex;

use common::bedrock_protocol::{RealmsApi, RealmsEnvironment};
use common::consts::bedrock::{
    BEDROCK_KEYRING_KEY_REFRESH_TOKEN, BEDROCK_KEYRING_KEY_XUID, XBOX_CLIENT_ID,
};
use common::structs::bedrock::{
    AddonMode, BedrockRenewal, BedrockStatus, NetworkInterface, ProtocolVersionOption, RealmEntry,
};
use common::traits::StreamTrait;

use crate::bedrock::ChatInjector;
use crate::bedrock::{
    BedrockAuthService, BedrockConnector, BedrockKeyringService, BedrockState,
    ProtocolVersionCatalog, ProxyConnectRequest, RealmConnectRequest,
};

#[tauri::command(async)]
pub(crate) async fn bedrock_start_proxy(
    app_handle: tauri::AppHandle,
    target_host: String,
    target_port: u16,
    listen_port: Option<u16>,
    network_interface: String,
    advertised_protocol: Option<u32>,
    addon_mode: Option<AddonMode>,
) -> Result<(), String> {
    BedrockConnector::new(app_handle)
        .start_proxy(ProxyConnectRequest {
            target_host,
            target_port,
            listen_port,
            network_interface: Some(network_interface),
            advertised_protocol,
            addon_mode,
        })
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command(async)]
pub(crate) async fn bedrock_stop_proxy(app_handle: tauri::AppHandle) -> Result<(), String> {
    BedrockConnector::new(app_handle)
        .stop_proxy()
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command(async)]
pub(crate) async fn bedrock_start_realms(
    app_handle: tauri::AppHandle,
    realm_id: u64,
    realm_name: String,
    network_interface: String,
) -> Result<(), String> {
    BedrockConnector::new(app_handle)
        .start_realm(RealmConnectRequest {
            realm_id,
            realm_name,
            network_interface: Some(network_interface),
        })
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command(async)]
pub(crate) async fn bedrock_stop_realms(app_handle: tauri::AppHandle) -> Result<(), String> {
    BedrockConnector::new(app_handle)
        .stop_realm()
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
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
    let auth_future = RealmsApi::authenticate(XBOX_CLIENT_ID, RealmsEnvironment::Retail, move |code, url| {
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
    let auth_manager =
        auth.build_auth_manager(refresh_token.as_deref(), &xuid, Some(&app_handle));
    state.apply_auth(
        auth_manager,
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

    let keyring = BedrockKeyringService::new(&app_handle);

    let refresh_token = match keyring.load(BEDROCK_KEYRING_KEY_REFRESH_TOKEN) {
        Some(rt) => rt,
        None => return Ok(false),
    };
    let stored_xuid = keyring.load(BEDROCK_KEYRING_KEY_XUID);

    // Only a credential the provider actually rejected is worth deleting. `authenticate_refresh`
    // reaches the token endpoint over the network, so an unreachable host arrives here too, and
    // discarding on that trades a device-code prompt for a dropped connection.
    let result = RealmsApi::authenticate_refresh(
        XBOX_CLIENT_ID,
        &refresh_token,
        RealmsEnvironment::Retail,
    )
    .await
    .map_err(|e| {
        if matches!(BedrockRenewal::from(&e), BedrockRenewal::ReauthRequired) {
            keyring.clear();
        }
        e.to_string()
    })?;

    let (api, xbl_token, user_hash, access_token, new_refresh_token) = result;
    let auth = BedrockAuthService::new();
    let xuid = match stored_xuid {
        Some(x) => x,
        None => auth
            .extract_xuid(&xbl_token)
            .await
            .map_err(|e| format!("Failed to extract XUID during auth restore: {}", e))?,
    };
    keyring.store(BEDROCK_KEYRING_KEY_XUID, &xuid);

    let effective_refresh = new_refresh_token.or(Some(refresh_token));
    if let Some(ref rt) = effective_refresh {
        keyring.store(BEDROCK_KEYRING_KEY_REFRESH_TOKEN, rt);
    }

    let auth_manager =
        auth.build_auth_manager(effective_refresh.as_deref(), &xuid, Some(&app_handle));

    let mut state = state.lock().await;
    state.apply_auth(
        auth_manager,
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
) -> Result<BedrockRenewal, String> {
    Ok(BedrockAuthService::new()
        .renew(state.inner(), &app_handle)
        .await)
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

    BedrockAuthService::new().sign_out(&mut state, &app_handle);
    Ok(())
}

#[tauri::command(async)]
pub(crate) async fn bedrock_list_protocol_versions() -> Result<Vec<ProtocolVersionOption>, String> {
    Ok(ProtocolVersionCatalog::released())
}

#[tauri::command(async)]
pub(crate) async fn bedrock_list_interfaces() -> Result<Vec<NetworkInterface>, String> {
    BedrockConnector::interfaces().map_err(|e| e.to_string())
}

#[tauri::command(async)]
pub(crate) async fn bedrock_list_realms(
    state: State<'_, Mutex<BedrockState>>,
    app_handle: tauri::AppHandle,
) -> Result<Vec<RealmEntry>, String> {
    let api = {
        let state = state.lock().await;
        state.realms_api.as_ref().cloned()
    };

    let listed = match api {
        Some(api) => api.list_worlds().await.map_err(|e| e.to_string()),
        None => Err(crate::bedrock::XBOX_AUTH_REQUIRED.to_string()),
    };

    // A stale XSTS token recovers here without anyone being told; only a rejected credential
    // reaches the player.
    let worlds = match listed {
        Ok(w) => w,
        Err(_) => {
            match BedrockAuthService::new()
                .renew(state.inner(), &app_handle)
                .await
            {
                BedrockRenewal::ReauthRequired => {
                    return Err(crate::bedrock::REAUTH_REQUIRED.to_string());
                }
                BedrockRenewal::Unavailable { message } => return Err(message),
                BedrockRenewal::Renewed => {}
            }

            let api = {
                let state = state.lock().await;
                state
                    .realms_api
                    .as_ref()
                    .cloned()
                    .ok_or_else(|| crate::bedrock::XBOX_AUTH_REQUIRED.to_string())?
            };
            api.list_worlds().await.map_err(|e| e.to_string())?
        }
    };

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
        xbox_authenticated: state.is_authenticated(),
        reauth_required: state.reauth_required,
        proxy_target_host: state.proxy_target_host.clone(),
        proxy_target_port: state.proxy_target_port,
        proxy_listen_port: state.proxy_listen_port,
        proxy_started_at: state.proxy_started_at,
        active_realm_id: state.active_realm_id,
        active_realm_name: state.active_realm_name.clone(),
        // Resolved once at connect time by the connector, which is also what names the entry in
        // Minecraft's Friends tab, so both surfaces call the world the same thing.
        active_connection_name: state
            .active_connection
            .as_ref()
            .map(|connection| connection.name.clone()),
    })
}

/// Sends a line from the app into the realm the proxy is connected to.
///
/// No-net only. The line is queued for the session loop, which injects it as ordinary chat
/// under the player's own name — see `BedrockProxyManager::inject_chat`.
#[tauri::command]
pub(crate) async fn bedrock_send_chat(
    text: String,
    state: State<'_, Mutex<BedrockState>>,
    chat_injector: State<'_, Arc<ChatInjector>>,
) -> Result<(), String> {
    let text = text.trim().to_string();
    if text.is_empty() {
        return Err("Nothing to send.".to_string());
    }

    let state = state.lock().await;
    let running = state
        .proxy
        .as_ref()
        .is_some_and(|p| !p.is_stopped())
        || state.realms.as_ref().is_some_and(|r| !r.is_stopped());

    if !running {
        return Err("Not connected to a world.".to_string());
    }

    // A full queue means the session loop is not draining. Reporting it beats a message that
    // silently disappears.
    if !chat_injector.enqueue(text) {
        return Err("Chat is backed up. Try again in a moment.".to_string());
    }

    Ok(())
}
