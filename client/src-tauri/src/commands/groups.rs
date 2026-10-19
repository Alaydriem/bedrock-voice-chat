use tauri::AppHandle;
use websocket_types::GroupData;

use crate::groups::GroupService;

/// Create a group and join it.
///
/// One of three thin wrappers over `GroupService`, which is the single entry point: the in-game
/// panel reaches groups through these commands and a WebSocket controller reaches the same service
/// directly, so both drive identical code.
#[tauri::command]
pub(crate) async fn group_create(app: AppHandle, name: String) -> Result<GroupData, String> {
    GroupService::new(app)
        .create(name)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn group_join(app: AppHandle, name: String) -> Result<GroupData, String> {
    GroupService::new(app)
        .join(name)
        .await
        .map_err(|e| e.to_string())
}

/// Create an end-to-end encrypted group. The password stays in this process.
#[tauri::command]
pub(crate) async fn group_create_protected(
    app: AppHandle,
    name: String,
    password: String,
) -> Result<GroupData, String> {
    GroupService::new(app)
        .create_protected(name, password)
        .await
        .map_err(|e| e.to_string())
}

/// Join an end-to-end encrypted group by id. Admission completes asynchronously and is reported
/// through `protected-group:*` events.
#[tauri::command]
pub(crate) async fn group_join_protected(
    app: AppHandle,
    id: String,
    password: String,
) -> Result<GroupData, String> {
    GroupService::new(app)
        .join_protected(id, password)
        .await
        .map_err(|e| e.to_string())
}

/// Stop this client's protected group and wipe its keys, before the webview leaves the channel.
/// The microphone stays silent until `group_release_protected`.
#[tauri::command]
pub(crate) async fn group_stop_protected(app: AppHandle) -> Result<(), String> {
    GroupService::new(app).stop_protected().await;
    Ok(())
}

/// Let audio flow unsealed again, once the webview has left the protected channel.
#[tauri::command]
pub(crate) async fn group_release_protected(app: AppHandle) -> Result<(), String> {
    GroupService::new(app).release_protected().await;
    Ok(())
}

#[tauri::command]
pub(crate) async fn group_leave(app: AppHandle) -> Result<GroupData, String> {
    GroupService::new(app)
        .leave()
        .await
        .map_err(|e| e.to_string())
}
