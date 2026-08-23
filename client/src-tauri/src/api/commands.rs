use crate::structs::app_state::AppState;
use common::consts::version::PROTOCOL_VERSION;
use common::response::ApiConfigCheckResponse;
use common::response::GamerpicResponse;
use common::structs::channel::{Channel, ChannelEvent};
use tauri::{State, async_runtime::Mutex};

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

/// Make a server callable without making it the server the app is signed in to.
///
/// For reading several servers' state at once. Every command afterwards has to name the
/// endpoint, because nothing here becomes the default.
#[tauri::command(async)]
pub(crate) async fn api_pool_client(
    app_state: State<'_, Mutex<AppState>>,
    endpoint: String,
    cert: String,
    pem: String,
) -> Result<(), String> {
    let state = app_state.lock().await;
    state.pool_api_client(endpoint, cert, pem).await;
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
            drop(state);
            app_state
                .lock()
                .await
                .get_api_client_for_server(&endpoint)
                .await?
        }
        None => state.get_api_client()?.clone(),
    };

    match api.ping().await {
        Ok(_) => Ok(()),
        Err(_) => Err("Ping failed".to_string()),
    }
}

#[tauri::command(async)]
pub(crate) async fn api_get_config(
    app_state: State<'_, Mutex<AppState>>,
    server: Option<String>,
) -> Result<ApiConfigCheckResponse, String> {
    let state = app_state.lock().await;

    let api = match server {
        Some(endpoint) => {
            drop(state);
            app_state
                .lock()
                .await
                .get_api_client_for_server(&endpoint)
                .await?
        }
        None => state.get_api_client()?.clone(),
    };

    let config = api.get_config().await?;
    Ok(ApiConfigCheckResponse::from_config(
        config,
        PROTOCOL_VERSION,
    ))
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
            app_state
                .lock()
                .await
                .get_api_client_for_server(&endpoint)
                .await?
        }
        None => state.get_api_client()?.clone(),
    };

    api.create_channel(name).await
}

#[tauri::command(async)]
pub(crate) async fn api_delete_channel(
    app_state: State<'_, Mutex<AppState>>,
    #[allow(non_snake_case)] channelId: String,
    server: Option<String>,
) -> Result<bool, String> {
    let state = app_state.lock().await;

    let api = match server {
        Some(endpoint) => {
            drop(state);
            app_state
                .lock()
                .await
                .get_api_client_for_server(&endpoint)
                .await?
        }
        None => state.get_api_client()?.clone(),
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
            app_state
                .lock()
                .await
                .get_api_client_for_server(&endpoint)
                .await?
        }
        None => state.get_api_client()?.clone(),
    };

    api.list_channels().await
}

#[tauri::command(async)]
pub(crate) async fn api_get_channel(
    app_state: State<'_, Mutex<AppState>>,
    #[allow(non_snake_case)] channelId: String,
    server: Option<String>,
) -> Result<Channel, String> {
    let state = app_state.lock().await;

    let api = match server {
        Some(endpoint) => {
            drop(state);
            app_state
                .lock()
                .await
                .get_api_client_for_server(&endpoint)
                .await?
        }
        None => state.get_api_client()?.clone(),
    };

    api.get_channel(&channelId).await
}

#[tauri::command(async)]
pub(crate) async fn api_channel_event(
    app_state: State<'_, Mutex<AppState>>,
    #[allow(non_snake_case)] channelId: String,
    event: ChannelEvent,
    server: Option<String>,
) -> Result<bool, String> {
    let state = app_state.lock().await;

    let api = match server {
        Some(endpoint) => {
            drop(state);
            app_state
                .lock()
                .await
                .get_api_client_for_server(&endpoint)
                .await?
        }
        None => state.get_api_client()?.clone(),
    };

    api.channel_event(channelId, event).await
}

#[tauri::command(async)]
pub(crate) async fn api_rename_channel(
    app_state: State<'_, Mutex<AppState>>,
    #[allow(non_snake_case)] channelId: String,
    name: String,
    server: Option<String>,
) -> Result<bool, String> {
    let state = app_state.lock().await;

    let api = match server {
        Some(endpoint) => {
            drop(state);
            app_state
                .lock()
                .await
                .get_api_client_for_server(&endpoint)
                .await?
        }
        None => state.get_api_client()?.clone(),
    };

    api.rename_channel(&channelId, &name).await
}

#[tauri::command(async)]
pub(crate) async fn api_get_player_gamerpic(
    app_state: State<'_, Mutex<AppState>>,
    game: common::Game,
    gamertag: String,
    server: Option<String>,
) -> Result<GamerpicResponse, String> {
    let state = app_state.lock().await;

    let api = match server {
        Some(endpoint) => {
            drop(state);
            app_state
                .lock()
                .await
                .get_api_client_for_server(&endpoint)
                .await?
        }
        None => state.get_api_client()?.clone(),
    };

    api.get_gamerpic(game.as_str(), &gamertag).await
}

/// Fetch a WebSocket ticket for a saved server.
///
/// The webview opens the socket itself — it cannot present a certificate, which is why the
/// ticket exists — so this is the one part of the handshake that has to happen in Rust.
#[tauri::command(async)]
pub(crate) async fn api_websocket_ticket(
    app_state: State<'_, Mutex<AppState>>,
    server: Option<String>,
) -> Result<common::response::websocket::WebsocketTicketResponse, String> {
    let state = app_state.lock().await;

    let api = match server {
        Some(endpoint) => {
            drop(state);
            app_state
                .lock()
                .await
                .get_api_client_for_server(&endpoint)
                .await?
        }
        None => state.get_api_client()?.clone(),
    };

    api.websocket_ticket().await
}
