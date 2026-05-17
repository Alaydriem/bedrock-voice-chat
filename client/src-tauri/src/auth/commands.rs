use crate::analytics::AnalyticsService;
use crate::auth::{code_login, hytale, login};
use crate::keyring::KeyringService;
use crate::structs::app_state::AppState;
use common::response::LinkJavaIdentityResponse;
use common::response::LoginResponse;
use common::structs::ServerListEntry;
use common::structs::config::{
    HytaleAuthStatus, HytaleDeviceFlowStartResponse, HytaleDeviceFlowStatusResponse,
};
use std::sync::Arc;
use tauri::{State, async_runtime::Mutex};
use tauri_plugin_store::StoreExt;

#[cfg(desktop)]
use crate::auth::mc_oauth_window::McOauthWindow;

#[tauri::command(async)]
#[tracing::instrument(skip(app_state, keyring, code))]
pub(crate) async fn server_login(
    app_state: State<'_, Mutex<AppState>>,
    keyring: State<'_, Mutex<KeyringService>>,
    server: String,
    code: String,
    redirect: String,
) -> Result<LoginResponse, bool> {
    let login_result = login::server_login(server.clone(), code, redirect).await;

    if let Ok(ref response) = login_result {
        let mut state = app_state.lock().await;
        state
            .initialize_api_client(
                server.clone(),
                response.certificate_ca.clone(),
                response.certificate.clone() + &response.certificate_key.clone(),
            )
            .await;

        let mut kr = keyring.lock().await;
        if let Err(e) = kr.store_credentials(&server, response) {
            log::error!("Failed to store credentials in keyring: {}", e);
        }
    }

    login_result
}

#[tauri::command(async)]
pub(crate) async fn logout(
    app_state: State<'_, Mutex<AppState>>,
    keyring: State<'_, Mutex<KeyringService>>,
    analytics: State<'_, Arc<AnalyticsService>>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    analytics.clear_connected_server();
    analytics.clear_player();
    let mut state = app_state.lock().await;

    // Get the current server before clearing it
    let current_server = state.current_server.clone();

    // Clear the API client
    state.clear_api_client();

    // Clear keyring credentials for the server
    if let Some(ref server_url) = current_server {
        let mut kr = keyring.lock().await;
        if let Err(e) = kr.delete_credentials(server_url) {
            log::warn!("Failed to clear keyring credentials: {}", e);
        }
    }

    // Get store and clear current session data
    let store = app_handle
        .store("store.json")
        .map_err(|e| format!("Failed to access store: {}", e))?;

    // Remove current_server and current_player from store
    store.delete("current_server");
    store.delete("current_player");

    if let Some(current_server_url) = current_server {
        if let Some(server_list_value) = store.get("server_list") {
            if let Ok(mut server_list) =
                serde_json::from_value::<Vec<ServerListEntry>>(server_list_value)
            {
                server_list.retain(|entry| entry.server != current_server_url);

                let updated_list = serde_json::to_value(server_list)
                    .map_err(|e| format!("Failed to serialize server list: {}", e))?;
                store.set("server_list", updated_list);
            }
        }
    }

    // Save the store
    store
        .save()
        .map_err(|e| format!("Failed to save store: {}", e))?;

    // Clear the current_server in AppState
    state.current_server = None;

    Ok(())
}

#[tauri::command(async)]
#[tracing::instrument(skip(app_state, keyring, code))]
pub(crate) async fn code_login(
    app_state: State<'_, Mutex<AppState>>,
    keyring: State<'_, Mutex<KeyringService>>,
    server: String,
    gamertag: String,
    code: String,
) -> Result<LoginResponse, String> {
    let login_result = code_login::code_login(server.clone(), gamertag, code)
        .await
        .map_err(|_| "Code login failed".to_string())?;

    let mut state = app_state.lock().await;
    state
        .initialize_api_client(
            server.clone(),
            login_result.certificate_ca.clone(),
            login_result.certificate.clone() + &login_result.certificate_key.clone(),
        )
        .await;

    let mut kr = keyring.lock().await;
    if let Err(e) = kr.store_credentials(&server, &login_result) {
        log::error!("Failed to store credentials in keyring: {}", e);
    }

    Ok(login_result)
}

#[tauri::command(async)]
pub(crate) async fn start_hytale_device_flow(
    server: String,
) -> Result<HytaleDeviceFlowStartResponse, bool> {
    hytale::start_hytale_device_flow(server).await
}

#[tauri::command(async)]
#[tracing::instrument(skip(app_state, keyring))]
pub(crate) async fn poll_hytale_status(
    app_state: State<'_, Mutex<AppState>>,
    keyring: State<'_, Mutex<KeyringService>>,
    server: String,
    session_id: String,
) -> Result<HytaleDeviceFlowStatusResponse, bool> {
    let poll_result = hytale::poll_hytale_status(server.clone(), session_id).await;

    if let Ok(ref response) = poll_result {
        if response.status == HytaleAuthStatus::Success {
            if let Some(ref login_response) = response.login_response {
                let mut state = app_state.lock().await;
                state
                    .initialize_api_client(
                        server.clone(),
                        login_response.certificate_ca.clone(),
                        login_response.certificate.clone()
                            + &login_response.certificate_key.clone(),
                    )
                    .await;

                let mut kr = keyring.lock().await;
                if let Err(e) = kr.store_credentials(&server, login_response) {
                    log::error!("Failed to store credentials in keyring: {}", e);
                }
            }
        }
    }

    poll_result
}

#[tauri::command(async)]
pub(crate) async fn refresh_server_state(
    app_state: State<'_, Mutex<AppState>>,
    keyring: State<'_, Mutex<KeyringService>>,
    server: Option<String>,
    game: Option<String>,
) -> Result<common::response::auth::AuthStateResponse, String> {
    let state = app_state.lock().await;
    let current_server = state.current_server.clone();
    let api = match server {
        Some(ref endpoint) => {
            drop(state);
            app_state
                .lock()
                .await
                .get_api_client_for_server(endpoint)
                .await?
        }
        None => {
            let api = state.get_api_client()?.clone();
            drop(state);
            api
        }
    };

    let response = api.get_server_state(game.as_deref()).await?;

    let target_server = server.or(current_server);
    if let Some(ref server_url) = target_server {
        let mut kr = keyring.lock().await;
        if let Ok(perms_json) = serde_json::to_string(&response.server_permissions) {
            let _ = kr.set_credential(server_url, "server_permissions", &perms_json);
        }

        if let (Some(cert), Some(cert_key)) =
            (&response.certificate, &response.certificate_key)
        {
            let _ = kr.set_credential(server_url, "certificate", cert);
            let _ = kr.set_credential(server_url, "certificate_key", cert_key);

            // Rebuild the API client with the rotated certificate
            let ca_cert = kr
                .get_credential(server_url, "certificate_ca")
                .map_err(|e| format!("Failed to get CA cert: {}", e))?;
            let pem = format!("{}{}", cert, cert_key);
            drop(kr);

            let mut state = app_state.lock().await;
            state
                .initialize_api_client(server_url.clone(), ca_cert, pem)
                .await;
        }
    }

    Ok(response)
}

#[tauri::command(async)]
pub(crate) async fn link_java_identity(
    app_handle: tauri::AppHandle,
    app_state: State<'_, Mutex<AppState>>,
    gamertag: String,
) -> Result<LinkJavaIdentityResponse, String> {

    #[cfg(desktop)]
    {
        let code = McOauthWindow::open(app_handle).await?;

        let state = app_state.lock().await;
        let api = state.get_api_client()?;
        return api.link_java_identity(
            code,
            McOauthWindow::redirect_uri().to_string(),
            McOauthWindow::client_id().to_string(),
            gamertag,
        )
        .await
    }

    return Err("Java identity linking is only available on the desktop app".to_string())
}
