use tauri::{AppHandle, State};
use tauri::async_runtime::Mutex;
use crate::websocket::{WebSocketManager, WebSocketConfig};
use common::traits::StreamTrait;

#[tauri::command]
pub async fn stop_websocket_server(
    ws_manager: State<'_, Mutex<WebSocketManager>>,
) -> Result<(), String> {
    #[cfg(desktop)]
    {
        let mut manager = ws_manager.lock().await;
        manager.stop().await.map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn is_websocket_running(
    ws_manager: State<'_, Mutex<WebSocketManager>>,
) -> Result<bool, String> {
    #[cfg(desktop)]
    {
        let manager = ws_manager.lock().await;
        Ok(!manager.is_stopped())
    }
    #[cfg(not(desktop))]
    {
        Ok(false)
    }
}

#[tauri::command]
pub async fn update_websocket_config(
    config: WebSocketConfig,
    ws_manager: State<'_, Mutex<WebSocketManager>>,
) -> Result<(), String> {
    let mut manager = ws_manager.lock().await;
    manager.update_config(config);
    Ok(())
}

#[tauri::command]
pub async fn start_websocket_server(
    ws_manager: State<'_, Mutex<WebSocketManager>>,
) -> Result<(), String> {
    #[cfg(desktop)]
    {
        let mut manager = ws_manager.lock().await;
        manager.start().await.map_err(|e| e.to_string())?;
    }
    Ok(())
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
