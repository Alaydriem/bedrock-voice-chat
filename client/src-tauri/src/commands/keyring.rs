use crate::keyring::KeyringService;
use common::response::LoginResponse;
use tauri::{State, async_runtime::Mutex};

#[tauri::command(async)]
pub(crate) async fn store_credentials(
    keyring: State<'_, Mutex<KeyringService>>,
    server: String,
    credentials: LoginResponse,
) -> Result<(), String> {
    let mut kr = keyring.lock().await;
    kr.store_credentials(&server, &credentials)
        .map_err(|e| e.to_string())
}

#[tauri::command(async)]
pub(crate) async fn get_credentials(
    keyring: State<'_, Mutex<KeyringService>>,
    server: String,
) -> Result<LoginResponse, String> {
    let mut kr = keyring.lock().await;
    kr.get_credentials(&server).map_err(|e| e.to_string())
}

#[tauri::command(async)]
pub(crate) async fn get_credential(
    keyring: State<'_, Mutex<KeyringService>>,
    server: String,
    key: String,
) -> Result<String, String> {
    let mut kr = keyring.lock().await;
    kr.get_credential(&server, &key)
        .map_err(|e| e.to_string())
}

#[tauri::command(async)]
pub(crate) async fn set_credential(
    keyring: State<'_, Mutex<KeyringService>>,
    server: String,
    key: String,
    value: String,
) -> Result<(), String> {
    let mut kr = keyring.lock().await;
    kr.set_credential(&server, &key, &value)
        .map_err(|e| e.to_string())
}

#[tauri::command(async)]
pub(crate) async fn delete_credentials(
    keyring: State<'_, Mutex<KeyringService>>,
    server: String,
) -> Result<(), String> {
    let mut kr = keyring.lock().await;
    kr.delete_credentials(&server)
        .map_err(|e| e.to_string())
}
