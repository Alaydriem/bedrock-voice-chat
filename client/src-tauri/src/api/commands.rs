use crate::structs::app_state::AppState;
use common::structs::channel::{Channel, ChannelEvent};
use tauri::{async_runtime::Mutex, State};

#[tauri::command(async)]
pub(crate) async fn api_initialize_client(
    app_state: State<'_, Mutex<AppState>>,
    endpoint: String,
    cert: String,
    pem: String,
) -> Result<(), String> {
    let mut state = app_state.lock().await;
    state.initialize_api_client(endpoint, cert, pem);
    Ok(())
}

#[tauri::command(async)]
pub(crate) async fn api_ping(app_state: State<'_, Mutex<AppState>>) -> Result<(), String> {
    let state = app_state.lock().await;
    match state.get_api_client() {
        Ok(api) => match api.ping().await {
            Ok(_) => Ok(()),
            Err(_) => Err("Ping failed".to_string()),
        },
        Err(e) => Err(e),
    }
}

#[tauri::command(async)]
pub(crate) async fn api_create_channel(
    app_state: State<'_, Mutex<AppState>>,
    name: String,
) -> Result<String, String> {
    let state = app_state.lock().await;
    match state.get_api_client() {
        Ok(api) => api.create_channel(name).await,
        Err(e) => Err(e),
    }
}

#[tauri::command(async)]
pub(crate) async fn api_delete_channel(
    app_state: State<'_, Mutex<AppState>>,
    #[allow(non_snake_case)]
    channelId: String,
) -> Result<bool, String> {
    let state = app_state.lock().await;
    match state.get_api_client() {
        Ok(api) => api.delete_channel(channelId).await,
        Err(e) => Err(e),
    }
}

#[tauri::command(async)]
pub(crate) async fn api_list_channels(
    app_state: State<'_, Mutex<AppState>>,
) -> Result<Vec<Channel>, String> {
    let state = app_state.lock().await;
    match state.get_api_client() {
        Ok(api) => api.list_channels().await,
        Err(e) => Err(e),
    }
}

#[tauri::command(async)]
pub(crate) async fn api_get_channel(
    app_state: State<'_, Mutex<AppState>>,
    #[allow(non_snake_case)]
    channelId: String,
) -> Result<Channel, String> {
    let state = app_state.lock().await;
    match state.get_api_client() {
        Ok(api) => api.get_channel(&channelId).await,
        Err(e) => Err(e),
    }
}

#[tauri::command(async)]
pub(crate) async fn api_channel_event(
    app_state: State<'_, Mutex<AppState>>,
    #[allow(non_snake_case)]
    channelId: String,
    event: ChannelEvent,
) -> Result<bool, String> {
    let state = app_state.lock().await;
    match state.get_api_client() {
        Ok(api) => api.channel_event(channelId, event).await,
        Err(e) => Err(e),
    }
}
