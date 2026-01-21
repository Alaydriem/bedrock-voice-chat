use crate::auth::{hytale, login};
use crate::structs::app_state::AppState;
use common::structs::config::{
    HytaleAuthStatus, HytaleDeviceFlowStartResponse, HytaleDeviceFlowStatusResponse, LoginResponse,
};
use tauri::{async_runtime::Mutex, State};
use tauri_plugin_store::StoreExt;

#[tauri::command(async)]
pub(crate) async fn server_login(
    app_state: State<'_, Mutex<AppState>>,
    server: String,
    code: String,
    redirect: String,
) -> Result<LoginResponse, bool> {
    let login_result = login::server_login(server.clone(), code, redirect).await;

    // If login is successful, initialize the API client
    if let Ok(ref response) = login_result {
        let mut state = app_state.lock().await;
        state.initialize_api_client(
            server,
            response.certificate_ca.clone(),
            response.certificate.clone() + &response.certificate_key.clone(),
        ).await;
    }

    login_result
}

#[tauri::command(async)]
pub(crate) async fn logout(
    app_state: State<'_, Mutex<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let mut state = app_state.lock().await;

    // Get the current server before clearing it
    let current_server = state.current_server.clone();

    // Clear the API client
    state.clear_api_client();

    // Get store and clear current session data
    let store = app_handle.store("store.json")
        .map_err(|e| format!("Failed to access store: {}", e))?;

    // Remove current_server and current_player from store
    store.delete("current_server");
    store.delete("current_player");

    // Remove the current server from server_list
    if let Some(current_server_url) = current_server {
        if let Some(server_list_value) = store.get("server_list") {
            if let Ok(mut server_list) = serde_json::from_value::<Vec<serde_json::Map<String, serde_json::Value>>>(server_list_value) {
                // Filter out the current server
                server_list.retain(|server_entry| {
                    server_entry.get("server")
                        .and_then(|v| v.as_str())
                        .map_or(true, |server_url| server_url != current_server_url)
                });

                // Save the updated server list
                let updated_list = serde_json::to_value(server_list)
                    .map_err(|e| format!("Failed to serialize server list: {}", e))?;
                store.set("server_list", updated_list);
            }
        }
    }

    // Save the store
    store.save().map_err(|e| format!("Failed to save store: {}", e))?;

    // Clear the current_server in AppState
    state.current_server = None;

    Ok(())
}

#[tauri::command(async)]
pub(crate) async fn start_hytale_device_flow(
    server: String,
) -> Result<HytaleDeviceFlowStartResponse, bool> {
    hytale::start_hytale_device_flow(server).await
}

#[tauri::command(async)]
pub(crate) async fn poll_hytale_status(
    app_state: State<'_, Mutex<AppState>>,
    server: String,
    session_id: String,
) -> Result<HytaleDeviceFlowStatusResponse, bool> {
    let poll_result = hytale::poll_hytale_status(server.clone(), session_id).await;

    // If login is successful, initialize the API client
    if let Ok(ref response) = poll_result {
        if response.status == HytaleAuthStatus::Success {
            if let Some(ref login_response) = response.login_response {
                let mut state = app_state.lock().await;
                state
                    .initialize_api_client(
                        server,
                        login_response.certificate_ca.clone(),
                        login_response.certificate.clone() + &login_response.certificate_key.clone(),
                    )
                    .await;
            }
        }
    }

    poll_result
}
