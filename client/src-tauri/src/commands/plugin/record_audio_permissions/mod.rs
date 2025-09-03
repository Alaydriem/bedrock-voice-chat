use tauri_plugin_audio_permissions::{AudioPermissionsExt, PermissionRequest};

#[tauri::command]
pub(crate) async fn request_audio_permission(app: tauri::AppHandle) -> Result<bool, String> {
    let response = app.audio_permissions()
        .request_permission(PermissionRequest {})
        .map_err(|e| e.to_string())?;
    Ok(response.granted)
}

#[tauri::command]
pub(crate) async fn check_audio_permission(app: tauri::AppHandle) -> Result<bool, String> {
    let response = app.audio_permissions()
        .check_permission(PermissionRequest {})
        .map_err(|e| e.to_string())?;
    Ok(response.granted)
}