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

#[tauri::command]
pub(crate) async fn group_leave(app: AppHandle) -> Result<GroupData, String> {
    GroupService::new(app)
        .leave()
        .await
        .map_err(|e| e.to_string())
}
