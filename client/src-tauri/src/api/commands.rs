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
    state.initialize_api_client(endpoint, cert, pem).await;
    Ok(())
}

#[tauri::command(async)]
pub(crate) async fn api_ping(
    app_state: State<'_, Mutex<AppState>>,
    server: Option<String>,
) -> Result<(), String> {
    let state = app_state.lock().await;

    let api = match server {
        Some(endpoint) => {
            // Use pool for specific server
            drop(state); // Release lock before async operation
            app_state.lock().await.get_api_client_for_server(&endpoint).await?
        },
        None => {
            // Use default client (backwards compatible)
            state.get_api_client()?.clone()
        }
    };

    match api.ping().await {
        Ok(_) => Ok(()),
        Err(_) => Err("Ping failed".to_string()),
    }
}

#[tauri::command(async)]
pub(crate) async fn api_create_channel(
    app_state: State<'_, Mutex<AppState>>,
    name: String,
    server: Option<String>,
) -> Result<String, String> {
    let state = app_state.lock().await;

    let api = match server {
        Some(endpoint) => {
            drop(state);
            app_state.lock().await.get_api_client_for_server(&endpoint).await?
        },
        None => {
            state.get_api_client()?.clone()
        }
    };

    api.create_channel(name).await
}

#[tauri::command(async)]
pub(crate) async fn api_delete_channel(
    app_state: State<'_, Mutex<AppState>>,
    #[allow(non_snake_case)]
    channelId: String,
    server: Option<String>,
) -> Result<bool, String> {
    let state = app_state.lock().await;

    let api = match server {
        Some(endpoint) => {
            drop(state);
            app_state.lock().await.get_api_client_for_server(&endpoint).await?
        },
        None => {
            state.get_api_client()?.clone()
        }
    };

    api.delete_channel(channelId).await
}

#[tauri::command(async)]
pub(crate) async fn api_list_channels(
    app_state: State<'_, Mutex<AppState>>,
    server: Option<String>,
) -> Result<Vec<Channel>, String> {
    let state = app_state.lock().await;

    let api = match server {
        Some(endpoint) => {
            drop(state);
            app_state.lock().await.get_api_client_for_server(&endpoint).await?
        },
        None => {
            state.get_api_client()?.clone()
        }
    };

    api.list_channels().await
}

#[tauri::command(async)]
pub(crate) async fn api_get_channel(
    app_state: State<'_, Mutex<AppState>>,
    #[allow(non_snake_case)]
    channelId: String,
    server: Option<String>,
) -> Result<Channel, String> {
    let state = app_state.lock().await;

    let api = match server {
        Some(endpoint) => {
            drop(state);
            app_state.lock().await.get_api_client_for_server(&endpoint).await?
        },
        None => {
            state.get_api_client()?.clone()
        }
    };

    api.get_channel(&channelId).await
}

#[tauri::command(async)]
pub(crate) async fn api_channel_event(
    app_state: State<'_, Mutex<AppState>>,
    #[allow(non_snake_case)]
    channelId: String,
    event: ChannelEvent,
    server: Option<String>,
) -> Result<bool, String> {
    let state = app_state.lock().await;

    let api = match server {
        Some(endpoint) => {
            drop(state);
            app_state.lock().await.get_api_client_for_server(&endpoint).await?
        },
        None => {
            state.get_api_client()?.clone()
        }
    };

    api.channel_event(channelId, event).await
}
