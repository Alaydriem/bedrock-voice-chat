use crate::AudioStreamManager;
use crate::analytics::AnalyticsService;
use crate::audio::recording::renderer::{
    AudioFormatRenderer, ExportNaming, SpatialRenderSettings,
};
use crate::audio::recording::{ExportRun, ManifestStore, TrackIndex, TrackSink};
use crate::audio::spatial::SpatialSettingsResolver;
use common::structs::AudioFormat;
use common::structs::recording::{
    ExportOutcome, ExportProgress, RecordingTrack, SessionManifest,
};
use common::structs::{AnalyticsEvent, AnalyticsEventData};
use log::{error, info};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::async_runtime::Mutex;
use tauri::{Emitter, Manager, State};
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
pub async fn get_recording_tracks(
    app_handle: tauri::AppHandle,
    session_id: String,
) -> Result<Vec<RecordingTrack>, String> {
    let session_path = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("recordings")
        .join(&session_id);

    TrackIndex::for_session(&session_path).map_err(|e| e.to_string())
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
pub async fn rename_recording_session(
    app_handle: tauri::AppHandle,
    session_id: String,
    name: String,
) -> Result<(), String> {
    let recordings_dir = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("recordings");

    ManifestStore::rename(&recordings_dir, &session_id, &name)
}

/// The run's one track at a time, against the session on disk.
struct SessionSink {
    app_handle: tauri::AppHandle,
    session_id: String,
    session_path: PathBuf,
    render_path: PathBuf,
    format: AudioFormat,
    // Absent when the export is flat. Resolved once for the whole run, because a session that
    // ends part way through must not change the curve the remaining tracks render on.
    spatial: Option<SpatialRenderSettings>,
}

#[async_trait::async_trait]
impl TrackSink for SessionSink {
    async fn write(&self, track: &RecordingTrack) -> Result<(), anyhow::Error> {
        let output_path = self.render_path.join(format!(
            "{}.{}",
            ExportNaming::file_stem(track),
            self.format.extension()
        ));
        self.format
            .render_track(
                &self.session_path,
                track,
                &output_path,
                self.spatial.as_ref(),
            )
            .await
    }

    fn progressed(&self, track: &RecordingTrack, index: u32, total: u32) {
        let _ = self.app_handle.emit(
            crate::events::event::RECORDING_EXPORT_PROGRESS,
            ExportProgress {
                session_id: self.session_id.clone(),
                track: track.display.clone(),
                index,
                total,
            },
        );
    }
}

#[tauri::command]
#[tracing::instrument(skip(app_handle, tracks, asm), fields(session_id = %session_id, format = ?format, track_count = tracks.len()))]
pub async fn export_recording(
    session_id: String,
    tracks: Vec<RecordingTrack>,
    spatial: bool,
    format: AudioFormat,
    app_handle: tauri::AppHandle,
    asm: State<'_, Mutex<AudioStreamManager>>,
) -> Result<ExportOutcome, String> {
    log::info!(
        "Export recording called - Session ID: {}, Tracks: {}, Spatial: {}, Format: {:?}",
        session_id,
        tracks.len(),
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
        return Err(
            "Recording was made with an incompatible version and cannot be exported".to_string(),
        );
    }

    let render_path = rec_path.join("renders");
    let _ = fs::create_dir_all(render_path.clone().to_path_buf());

    let export_format = format!("{:?}", format);
    let render_start = std::time::Instant::now();

    // Resolved before the run starts, and the lock released before any rendering: a render
    // decodes a whole session and must never hold the audio manager while it does.
    let spatial = if spatial {
        let live = {
            let asm = asm.lock().await;
            SpatialSettingsResolver::live(&asm).await
        };
        let settings = SpatialSettingsResolver::choose(
            live,
            SpatialSettingsResolver::last_known(&app_handle),
        );

        info!(
            "Spatial export using {} settings, falloff {}",
            settings.provenance().as_str(),
            settings.config().falloff_distance
        );

        Some(settings)
    } else {
        None
    };

    let sink = SessionSink {
        app_handle: app_handle.clone(),
        session_id: session_id.clone(),
        session_path: rec_path.clone(),
        render_path: render_path.clone(),
        format,
        spatial,
    };
    let outcome = ExportRun::execute(&tracks, &sink).await;

    for failure in &outcome.failed {
        error!("Error rendering {}: {}", failure.track, failure.reason);
    }
    info!("Rendered {} of {} tracks", outcome.written.len(), tracks.len());

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
        .insert("export_count", outcome.written.len() as u64)
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

    // The folder opens when anything at all landed in it. Opening an empty one on a total
    // failure hands somebody a directory to search instead of a reason.
    if !outcome.written.is_empty() {
        let _ = app_handle
            .opener()
            .open_path(render_path.to_string_lossy().to_string(), None::<&str>);
    }

    Ok(outcome)
}
