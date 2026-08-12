use common::structs::app::AppInfo;
use log::{info, warn};
use std::env;
use std::fs;
use std::sync::Arc;
use tauri::async_runtime::Mutex;
use tauri::{Manager, State};
use tauri_plugin_opener::OpenerExt;

use crate::analytics::{AnalyticsService, PlatformId};
use crate::commands::env::get_variant;
use crate::feature_flags::FeatureFlagService;
use crate::logging::{SentryLogger, Telemetry};
use crate::structs::app_state::AppState;

#[tauri::command]
pub(crate) fn get_app_info() -> AppInfo {
    AppInfo {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        protocol_version: common::consts::version::PROTOCOL_VERSION.to_string(),
        build_commit: env!("BUILD_COMMIT").to_string(),
        build_variant: get_variant(),
        build_number: option_env!("APP_BUILD_NUMBER").unwrap_or("local").to_string(),
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
    store
        .save()
        .map_err(|e| format!("Failed to save telemetry setting: {}", e))?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn get_platform_id(
    platform_id: State<'_, Arc<PlatformId>>,
) -> Result<String, String> {
    Ok(platform_id.get())
}

/// Replaces the anonymous identity everything reports under, and returns the
/// replacement.
///
/// The store is written before the live value changes, so a failed save leaves the
/// session reporting under the id that is still on disk rather than one that would
/// disappear at the next launch. Everything after the swap is best effort: the
/// identity has already changed by then, and reporting failure would leave the screen
/// showing an id the session has stopped using.
#[tauri::command]
pub(crate) async fn refresh_platform_id(
    platform_id: State<'_, Arc<PlatformId>>,
    analytics: State<'_, Arc<AnalyticsService>>,
    feature_flags: State<'_, Arc<FeatureFlagService>>,
    state: State<'_, Mutex<AppState>>,
) -> Result<String, String> {
    let next = PlatformId::generate();

    let store = state.lock().await.get_store().clone();
    store.set("install_id", next.clone());
    store
        .save()
        .map_err(|e| format!("Failed to save platform ID: {}", e))?;

    platform_id.set(next.clone());
    analytics.set_user(&next);
    info!("Platform ID replaced: {}", next);

    // Registers the new identity with Flagsmith and re-evaluates every flag under it.
    // Without this the session keeps the segments the retired id was in until the
    // hourly poll, which is what a failure here falls back to.
    if let Err(e) = feature_flags.refresh().await {
        warn!("Feature flags did not re-evaluate under the new platform ID: {}", e);
    }

    Ok(next)
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
