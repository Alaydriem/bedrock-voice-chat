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

/// Turn this player's own group into an end-to-end encrypted one, while they are alone in it.
/// The error is the server's refusal code: `not_alone`, `already_protected`, `not_creator` or
/// `not_found`.
#[tauri::command]
pub(crate) async fn group_protect(app: AppHandle, id: String) -> Result<GroupData, String> {
    GroupService::new(app)
        .protect(id)
        .await
        .map_err(|e| e.to_string())
}

/// Ask to join an end-to-end encrypted group by id. Admission completes asynchronously and is
/// reported through `protected-group:*` events.
#[tauri::command]
pub(crate) async fn group_join_protected(
    app: AppHandle,
    id: String,
) -> Result<GroupData, String> {
    GroupService::new(app)
        .join_protected(id)
        .await
        .map_err(|e| e.to_string())
}

/// The member pressed "Check code" on a join request card.
#[tauri::command]
pub(crate) async fn group_admission_check(
    app: AppHandle,
    joiner_socket: u64,
) -> Result<(), String> {
    GroupService::new(app)
        .check(joiner_socket)
        .await
        .map_err(|e| e.to_string())
}

/// The member pressed Admit or Deny on the card showing `check_id` (32 hex characters).
#[tauri::command]
pub(crate) async fn group_admission_decide(
    app: AppHandle,
    joiner_socket: u64,
    check_id: String,
    allow: bool,
) -> Result<(), String> {
    GroupService::new(app)
        .decide(joiner_socket, &check_id, allow)
        .await
        .map_err(|e| e.to_string())
}

/// The joiner pressed "It matches" or "No match" for `check_id`.
#[tauri::command]
pub(crate) async fn group_admission_confirm(
    app: AppHandle,
    check_id: String,
    matches: bool,
) -> Result<(), String> {
    GroupService::new(app)
        .confirm(&check_id, matches)
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
