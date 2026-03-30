use crate::analytics::AnalyticsService;
use crate::audio::recording::renderer::AudioFormatRenderer;
use common::structs::AudioFormat;
use common::structs::recording::SessionManifest;
use common::structs::{AnalyticsEvent, AnalyticsEventData};
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::Manager;
use tauri_plugin_opener::OpenerExt;

use common::structs::recording::RecordingSession;

struct DirectorySize;

impl DirectorySize {
    fn calculate(path: &PathBuf) -> Result<u64, std::io::Error> {
        let mut total_size = 0u64;

        if path.is_dir() {
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    total_size += Self::calculate(&path)?;
                } else {
                    total_size += entry.metadata()?.len();
                }
            }
        }

        Ok(total_size)
    }
}

#[tauri::command]
pub async fn get_recording_sessions(
    app_handle: tauri::AppHandle,
) -> Result<Vec<RecordingSession>, String> {
    let recordings_dir = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("recordings");

    if !recordings_dir.exists() {
        return Ok(vec![]);
    }

    let mut sessions = Vec::new();

    let entries = fs::read_dir(&recordings_dir)
        .map_err(|e| format!("Failed to read recordings directory: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let session_dir = entry.path();

        if !session_dir.is_dir() {
            continue;
        }

        let session_json_path = session_dir.join("session.json");
        if !session_json_path.exists() {
            continue;
        }

        // Read and parse session.json
        let session_json = fs::read_to_string(&session_json_path)
            .map_err(|e| format!("Failed to read session.json: {}", e))?;

        let session_data: SessionManifest = serde_json::from_str(&session_json)
            .map_err(|e| format!("Failed to parse session.json: {}", e))?;

        // Calculate directory size
        let size_bytes = DirectorySize::calculate(&session_dir)
            .map_err(|e| format!("Failed to calculate directory size: {}", e))?;

        let file_size_mb = size_bytes as f64 / (1024.0 * 1024.0);

        let exportable = session_data
            .recording_version
            .as_deref()
            .is_some_and(|v| v == common::consts::version::RECORDING_VERSION);

        let recording_session = RecordingSession {
            session_data,
            file_size_mb,
            recording_path: session_dir.to_string_lossy().to_string(),
            exportable,
        };

        sessions.push(recording_session);
    }

    sessions.sort_by(|a, b| {
        b.session_data
            .start_timestamp
            .cmp(&a.session_data.start_timestamp)
    });

    Ok(sessions)
}

#[tauri::command]
pub async fn delete_recording_session(
    app_handle: tauri::AppHandle,
    session_id: String,
) -> Result<bool, String> {
    let recordings_dir = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("recordings")
        .join(&session_id);

    if !recordings_dir.exists() {
        return Err("Recording session not found".to_string());
    }

    fs::remove_dir_all(&recordings_dir)
        .map_err(|e| format!("Failed to delete recording directory: {}", e))?;

    Ok(true)
}

#[tauri::command]
#[tracing::instrument(skip(app_handle, selected_players), fields(session_id = %session_id, format = ?format, player_count = selected_players.len()))]
pub async fn export_recording(
    session_id: String,
    selected_players: Vec<String>,
    spatial: bool,
    format: AudioFormat,
    app_handle: tauri::AppHandle,
) -> Result<bool, String> {
    log::info!(
        "Export recording called - Session ID: {}, Players: {}, Spatial: {}, Format: {:?}",
        session_id,
        selected_players.len(),
        spatial,
        format
    );

    let rec_path = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("recordings")
        .join(&session_id);

    let session_json_path = rec_path.join("session.json");
    let session_manifest: Option<SessionManifest> = fs::read_to_string(&session_json_path)
        .ok()
        .and_then(|json| serde_json::from_str(&json).ok());

    let exportable = session_manifest
        .as_ref()
        .and_then(|m| m.recording_version.as_deref())
        .is_some_and(|v| v == common::consts::version::RECORDING_VERSION);

    if !exportable {
        return Err("Recording was made with an incompatible version and cannot be exported".to_string());
    }

    let session_path = rec_path.clone();
    let render_path = rec_path.join("renders");
    let _ = fs::create_dir_all(render_path.clone().to_path_buf());
    let render_path_for_open = render_path.clone();

    let export_player_count = selected_players.len();
    let export_format = format!("{:?}", format);
    let render_start = std::time::Instant::now();

    let task = tokio::spawn({
        use tracing::Instrument;
        async move {
            for (index, player) in selected_players.iter().enumerate() {
                let span = tracing::info_span!("render_player", index = index);
                let output_path = render_path.join(format!("{}.{}", player, format.extension()));
                async {
                    match format.render(&session_path, player, &output_path).await {
                        Ok(()) => {
                            info!("Rendered player {}", index);
                        }
                        Err(e) => {
                            error!("Error rendering player {}: {}", index, e);
                        }
                    }
                }
                .instrument(span)
                .await;
            }
        }
        .instrument(tracing::Span::current())
    });

    match task.await {
        Ok(()) => {
            let render_time_ms = render_start.elapsed().as_millis() as u64;

            let analytics = app_handle.state::<Arc<AnalyticsService>>();
            let event_data = AnalyticsEventData::new()
                .insert(
                    "participant_count",
                    session_manifest
                        .as_ref()
                        .map(|m| m.participants.len() as u64)
                        .unwrap_or(0),
                )
                .insert("export_count", export_player_count as u64)
                .insert(
                    "duration_ms",
                    session_manifest
                        .as_ref()
                        .and_then(|m| m.duration_ms)
                        .unwrap_or(0),
                )
                .insert("format", export_format)
                .insert("render_time_ms", render_time_ms);
            analytics.track(AnalyticsEvent::RecordingExported, Some(event_data));

            let _ = app_handle.opener().open_path(
                render_path_for_open.to_string_lossy().to_string(),
                None::<&str>,
            );
        }
        Err(e) => {
            error!("JoinHandler for Recording failed to join, {}", e);
        }
    };

    Ok(true)
}
