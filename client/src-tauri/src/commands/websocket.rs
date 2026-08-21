use crate::websocket::{WebSocketConfig, WebSocketManager};
use common::structs::websocket::{InternalEndpoint, WebSocketClientInfo};
use common::traits::StreamTrait;
use tauri::State;
use tauri::async_runtime::Mutex;

// Available on mobile as well as desktop. The process survives backgrounding through the
// Android audio foreground service and the iOS `audio` background mode.

/// Rebind the operator-facing listener after its port, key or reach changed.
///
/// The internal listener is untouched. It has no user-visible settings, and rebinding it would
/// drop the meters for a change that has nothing to do with them.
///
/// Answers with the port it landed on rather than nothing. The configured port is a preference,
/// so the caller cannot assume the setting it just saved is the address to hand out.
#[tauri::command]
pub async fn restart_websocket_external(
    ws_manager: State<'_, Mutex<WebSocketManager>>,
) -> Result<u16, String> {
    let mut manager = ws_manager.lock().await;
    manager.stop().await.map_err(|e| e.to_string())?;
    manager.start().await.map_err(|e| e.to_string())?;
    manager
        .external_port()
        .ok_or_else(|| "the WebSocket server started but reported no port".to_string())
}

#[tauri::command]
pub async fn update_websocket_config(
    config: WebSocketConfig,
    ws_manager: State<'_, Mutex<WebSocketManager>>,
) -> Result<(), String> {
    // The last place that sees the config before it binds. There is no disabled state left, so
    // a token is a requirement rather than a condition of one.
    if config.key.trim().is_empty() {
        return Err("The WebSocket server needs an access token".to_string());
    }

    let mut manager = ws_manager.lock().await;
    manager.update_config(config);
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

/// Where this process's push channel is listening.
///
/// Errors while the listener is still binding rather than returning a placeholder, so the caller
/// retries instead of dialling a port that is not there.
#[tauri::command]
pub async fn websocket_internal_endpoint(
    ws_manager: State<'_, Mutex<WebSocketManager>>,
) -> Result<InternalEndpoint, String> {
    let manager = ws_manager.lock().await;
    manager
        .internal_endpoint()
        .ok_or_else(|| "the internal push listener is not bound yet".to_string())
}
