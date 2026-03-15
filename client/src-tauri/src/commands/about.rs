use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{Manager, State};
use tauri_plugin_opener::OpenerExt;
use log::info;
use std::fs;
use tauri::async_runtime::Mutex;

use crate::commands::env::get_variant;
use crate::logging::{SentryLogger, Telemetry};
use crate::structs::app_state::AppState;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppInfo {
    pub app_version: String,
    pub protocol_version: String,
    pub build_commit: String,
    pub build_variant: String,
}

#[tauri::command]
pub(crate) fn get_app_info() -> AppInfo {
    AppInfo {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        protocol_version: common::consts::version::PROTOCOL_VERSION.to_string(),
        build_commit: env!("BUILD_COMMIT").to_string(),
        build_variant: get_variant(),
    }
}

#[tauri::command]
pub(crate) async fn get_telemetry(telemetry: State<'_, Arc<Telemetry>>) -> Result<bool, String> {
    Ok(telemetry.is_enabled())
}

#[tauri::command]
pub(crate) async fn set_telemetry(
    value: bool,
    telemetry: State<'_, Arc<Telemetry>>,
    sentry_logger: State<'_, Arc<SentryLogger>>,
    state: State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    telemetry.set(value);
    sentry_logger.set(value);

    let store = state.lock().await.get_store().clone();
    store.set("telemetry", value);
    store.save().map_err(|e| format!("Failed to save telemetry setting: {}", e))?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn export_logs(app_handle: tauri::AppHandle) -> Result<bool, String> {
    let log_dir = app_handle
        .path()
        .app_log_dir()
        .map_err(|e| format!("Failed to get log directory: {}", e))?;

    if !log_dir.exists() {
        return Err("Log directory does not exist".to_string());
    }

    let tar_data = {
        let mut tar_builder = tar::Builder::new(Vec::new());
        tar_builder
            .append_dir_all("logs", &log_dir)
            .map_err(|e| format!("Failed to create tar archive: {}", e))?;
        tar_builder
            .into_inner()
            .map_err(|e| format!("Failed to finalize tar archive: {}", e))?
    };

    let compressed = zstd::encode_all(tar_data.as_slice(), 3)
        .map_err(|e| format!("Failed to compress logs: {}", e))?;

    let export_dir = app_handle
        .path()
        .app_cache_dir()
        .map_err(|e| format!("Failed to get cache directory: {}", e))?
        .join("exports");

    fs::create_dir_all(&export_dir)
        .map_err(|e| format!("Failed to create export directory: {}", e))?;

    let export_path = export_dir.join("bvc-logs.tar.zst");

    fs::write(&export_path, &compressed)
        .map_err(|e| format!("Failed to write log archive: {}", e))?;

    info!("Exported logs to {:?}", export_path);

    let _ = app_handle
        .opener()
        .open_path(export_dir.to_string_lossy().to_string(), None::<&str>);

    Ok(true)
}
