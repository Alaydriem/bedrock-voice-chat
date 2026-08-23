use std::sync::Arc;

use common::structs::audio::PlayerGainSettings;
use common::structs::players::PlayerSettingsRow;
use tauri::{AppHandle, State};

use crate::players::PlayerSettingsCoordinator;

type Coordinator<'a> = State<'a, Arc<PlayerSettingsCoordinator>>;

#[tauri::command(async)]
pub(crate) async fn player_settings_list(
    app: AppHandle,
    coordinator: Coordinator<'_>,
) -> Result<Vec<PlayerSettingsRow>, String> {
    coordinator.list(&app).await.map_err(|e| e.to_string())
}

#[tauri::command(async)]
pub(crate) async fn player_settings_set_gain(
    app: AppHandle,
    cn: String,
    gain: f32,
    coordinator: Coordinator<'_>,
) -> Result<(), String> {
    coordinator
        .set_gain(&app, &cn, gain)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command(async)]
pub(crate) async fn player_settings_set_muted(
    app: AppHandle,
    cn: String,
    muted: bool,
    coordinator: Coordinator<'_>,
) -> Result<(), String> {
    coordinator
        .set_muted(&app, &cn, muted)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command(async)]
pub(crate) async fn player_settings_forget(
    app: AppHandle,
    cn: String,
    coordinator: Coordinator<'_>,
) -> Result<(), String> {
    coordinator
        .forget(&app, &cn)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command(async)]
pub(crate) async fn player_settings_reset_all(
    app: AppHandle,
    coordinator: Coordinator<'_>,
) -> Result<(), String> {
    coordinator.reset_all(&app).await.map_err(|e| e.to_string())
}

#[tauri::command(async)]
pub(crate) async fn player_settings_touch(
    app: AppHandle,
    cn: String,
    coordinator: Coordinator<'_>,
) -> Result<PlayerGainSettings, String> {
    coordinator.touch(&app, &cn).await.map_err(|e| e.to_string())
}

/// Seeds the mixer with the persisted projection once the server is known.
///
/// Until it runs, the mixer's projection is empty and every persisted mute is inert.
#[tauri::command(async)]
pub(crate) async fn player_settings_publish(
    app: AppHandle,
    coordinator: Coordinator<'_>,
) -> Result<(), String> {
    coordinator.publish(&app, None).await.map_err(|e| e.to_string())
}
