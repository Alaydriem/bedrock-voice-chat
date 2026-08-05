use crate::websocket::{WebSocketConfig, WebSocketManager};
use common::structs::websocket::WebSocketClientInfo;
use common::traits::StreamTrait;
use tauri::State;
use tauri::async_runtime::Mutex;

// Available on mobile as well as desktop. The process survives backgrounding through the
// Android audio foreground service and the iOS `audio` background mode.

#[tauri::command]
pub async fn stop_websocket_server(
    ws_manager: State<'_, Mutex<WebSocketManager>>,
) -> Result<(), String> {
    let mut manager = ws_manager.lock().await;
    manager.stop().await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn is_websocket_running(
    ws_manager: State<'_, Mutex<WebSocketManager>>,
) -> Result<bool, String> {
    let manager = ws_manager.lock().await;
    Ok(!manager.is_stopped())
}

#[tauri::command]
pub async fn update_websocket_config(
    config: WebSocketConfig,
    ws_manager: State<'_, Mutex<WebSocketManager>>,
) -> Result<(), String> {
    // The last place that sees the config before it binds.
    if config.enabled && config.key.trim().is_empty() {
        return Err("The WebSocket server needs an access token before it can start".to_string());
    }

    let mut manager = ws_manager.lock().await;
    manager.update_config(config);
    Ok(())
}

#[tauri::command]
pub async fn start_websocket_server(
    ws_manager: State<'_, Mutex<WebSocketManager>>,
) -> Result<(), String> {
    let mut manager = ws_manager.lock().await;
    manager.start().await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn websocket_clients(
    ws_manager: State<'_, Mutex<WebSocketManager>>,
) -> Result<Vec<WebSocketClientInfo>, String> {
    let manager = ws_manager.lock().await;
    Ok(manager.clients().snapshot())
}

#[tauri::command]
pub async fn generate_encryption_key() -> Result<String, String> {
    use rand::RngExt;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::rng();

    let key: String = (0..32)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect();

    Ok(key)
}
