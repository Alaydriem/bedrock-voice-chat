use crate::analytics::AnalyticsService;
use crate::auth::{AuthClient, hytale};
use crate::keyring::KeyringService;
use crate::structs::app_state::AppState;
use common::response::LinkJavaIdentityResponse;
use common::response::LoginResponse;
use common::structs::config::{
    HytaleAuthStatus, HytaleDeviceFlowStartResponse, HytaleDeviceFlowStatusResponse,
};
use std::sync::Arc;
use tauri::{State, async_runtime::Mutex};

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
) -> Result<LoginResponse, String> {
    let login_result = AuthClient::server_login(server.clone(), code, redirect).await;

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
            return Err(e.to_string());
        }
    }

    login_result.map_err(|e| e.to_string())
}

#[tauri::command(async)]
pub(crate) async fn logout(
    app_state: State<'_, Mutex<AppState>>,
    keyring: State<'_, Mutex<KeyringService>>,
    analytics: State<'_, Arc<AnalyticsService>>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let mut state = app_state.lock().await;
    let mut kr = keyring.lock().await;

    crate::auth::SessionService::new(app_handle)
        .forget_current_server(&mut state, &mut kr, &analytics)
        .await
}

#[tauri::command(async)]
#[tracing::instrument(skip(app_state, keyring, code))]
pub(crate) async fn code_login(
    app_state: State<'_, Mutex<AppState>>,
    keyring: State<'_, Mutex<KeyringService>>,
    server: String,
    code: String,
) -> Result<LoginResponse, String> {
    let login_result = AuthClient::code_login(server.clone(), code)
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
        return Err(e.to_string());
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
                    return Err(false);
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

        if let (Some(cert), Some(cert_key)) = (&response.certificate, &response.certificate_key) {
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
        return api
            .link_java_identity(
                code,
                McOauthWindow::redirect_uri().to_string(),
                McOauthWindow::client_id().to_string(),
                gamertag,
            )
            .await;
    }

    // Reachable on non-desktop platforms. Supress linter warning
    #[allow(unused)]
    return Err("Java identity linking is only available on the desktop app".to_string());
}
