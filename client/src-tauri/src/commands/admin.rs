use common::Game;
use common::request::admin::AdminUserListQuery;
use common::response::PaginatedResponse;
use common::response::admin::{AdminActionOutcome, AdminUserRow, PermissionListResponse};
use common::response::auth::IntrospectResponse;
use common::structs::permission::{PermissionEffect, ServerPermissions};
use tauri::{State, async_runtime::Mutex};

use crate::keyring::KeyringService;
use crate::structs::app_state::AppState;

#[tauri::command(async)]
pub(crate) async fn admin_list_users(
    app_state: State<'_, Mutex<AppState>>,
    server: Option<String>,
    query: Option<AdminUserListQuery>,
) -> Result<PaginatedResponse<AdminUserRow>, String> {
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

    api.admin_list_users(&query.unwrap_or_default()).await
}

#[tauri::command(async)]
pub(crate) async fn admin_create_user(
    app_state: State<'_, Mutex<AppState>>,
    gamertag: String,
    game: Game,
    server: Option<String>,
) -> Result<AdminActionOutcome, String> {
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

    api.admin_create_user(&gamertag, &game).await
}

#[tauri::command(async)]
pub(crate) async fn admin_set_banished(
    app_state: State<'_, Mutex<AppState>>,
    gamertag: String,
    game: Game,
    banish: bool,
    server: Option<String>,
) -> Result<AdminActionOutcome, String> {
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

    api.admin_set_banished(&gamertag, &game, banish).await
}

#[tauri::command(async)]
pub(crate) async fn admin_list_permissions(
    app_state: State<'_, Mutex<AppState>>,
    gamertag: String,
    game: Game,
    server: Option<String>,
) -> Result<PermissionListResponse, String> {
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

    api.admin_list_permissions(&gamertag, &game).await
}

#[tauri::command(async)]
pub(crate) async fn admin_set_permission(
    app_state: State<'_, Mutex<AppState>>,
    gamertag: String,
    game: Game,
    permission: String,
    effect: PermissionEffect,
    server: Option<String>,
) -> Result<AdminActionOutcome, String> {
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

    api.admin_set_permission(&gamertag, &game, &permission, effect)
        .await
}

#[tauri::command(async)]
pub(crate) async fn admin_clear_permission(
    app_state: State<'_, Mutex<AppState>>,
    gamertag: String,
    game: Game,
    permission: String,
    server: Option<String>,
) -> Result<AdminActionOutcome, String> {
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

    api.admin_clear_permission(&gamertag, &game, &permission)
        .await
}

/// The caller's own permissions, refreshed from the server.
///
/// Writes the refreshed set back into the keyring under `server_permissions` — the key the
/// login already populates and the settings gate already reads. Without the write-back, a
/// permission granted mid-session stays invisible until the next login.
#[tauri::command(async)]
pub(crate) async fn api_introspect(
    app_state: State<'_, Mutex<AppState>>,
    keyring: State<'_, Mutex<KeyringService>>,
    server: Option<String>,
) -> Result<IntrospectResponse, String> {
    let state = app_state.lock().await;
    let api = match server.clone() {
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

    let endpoint = server.unwrap_or_else(|| api.endpoint().to_string());
    let response = api.introspect().await?;

    let permissions = ServerPermissions {
        allowed: response.permissions.clone(),
    };
    if let Ok(json) = serde_json::to_string(&permissions) {
        let mut kr = keyring.lock().await;
        if let Err(e) = kr.set_credential(&endpoint, "server_permissions", &json) {
            log::warn!("api_introspect: failed to cache permissions: {}", e);
        }
    }

    Ok(response)
}
