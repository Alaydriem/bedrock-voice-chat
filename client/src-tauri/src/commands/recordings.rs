use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tauri::Manager;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PlayerAudioConfig {
    pub sample_rate: u32,
    pub channels: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SessionData {
    pub session_id: String,
    pub start_timestamp: u64,
    pub end_timestamp: Option<u64>,
    pub duration_ms: Option<u64>,
    pub emitter_player: String,
    pub participants: Vec<String>,
    pub player_audio_configs: HashMap<String, PlayerAudioConfig>,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RecordingSession {
    pub session_data: SessionData,
    pub file_size_mb: f64,
    pub recording_path: String,
}

fn get_directory_size(path: &PathBuf) -> Result<u64, std::io::Error> {
    let mut total_size = 0u64;

    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                total_size += get_directory_size(&path)?;
            } else {
                total_size += entry.metadata()?.len();
            }
        }
    }

    Ok(total_size)
}

#[tauri::command]
pub async fn get_recording_sessions(app_handle: tauri::AppHandle) -> Result<Vec<RecordingSession>, String> {
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

        let session_data: SessionData = serde_json::from_str(&session_json)
            .map_err(|e| format!("Failed to parse session.json: {}", e))?;

        // Calculate directory size
        let size_bytes = get_directory_size(&session_dir)
            .map_err(|e| format!("Failed to calculate directory size: {}", e))?;

        let file_size_mb = size_bytes as f64 / (1024.0 * 1024.0);

        let recording_session = RecordingSession {
            session_data,
            file_size_mb,
            recording_path: session_dir.to_string_lossy().to_string(),
        };

        sessions.push(recording_session);
    }

    // Sort by start timestamp (newest first)
    sessions.sort_by(|a, b| b.session_data.start_timestamp.cmp(&a.session_data.start_timestamp));

    Ok(sessions)
}

#[tauri::command]
pub async fn delete_recording_session(app_handle: tauri::AppHandle, session_id: String) -> Result<bool, String> {
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