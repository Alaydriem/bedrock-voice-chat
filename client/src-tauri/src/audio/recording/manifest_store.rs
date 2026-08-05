use std::fs;
use std::path::{Path, PathBuf};

use common::structs::recording::SessionManifest;

// Reads and writes a session's `session.json`.
//
// The manifest is the only mutable thing about a finished recording, and the audio
// beside it is irreplaceable. So a write goes to a sibling temp file and is renamed
// over the original: a process that dies mid-write leaves the old manifest intact
// rather than a truncated one that makes the whole session unreadable.
pub struct ManifestStore;

impl ManifestStore {
    pub fn rename(recordings_dir: &Path, session_id: &str, name: &str) -> Result<(), String> {
        let session_dir = recordings_dir.join(session_id);
        let manifest_path = session_dir.join("session.json");
        if !manifest_path.exists() {
            return Err("Recording session not found".to_string());
        }

        let mut manifest = Self::read(&manifest_path)?;
        let trimmed = name.trim();
        // Clearing the name is a legitimate outcome of emptying the field, and it puts
        // the row back to the time it was recorded rather than leaving it blank.
        manifest.name = if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        };

        Self::write(&manifest_path, &manifest)
    }

    fn read(path: &PathBuf) -> Result<SessionManifest, String> {
        let json =
            fs::read_to_string(path).map_err(|e| format!("Failed to read session.json: {}", e))?;
        serde_json::from_str(&json).map_err(|e| format!("Failed to parse session.json: {}", e))
    }

    fn write(path: &PathBuf, manifest: &SessionManifest) -> Result<(), String> {
        let json = serde_json::to_string_pretty(manifest)
            .map_err(|e| format!("Failed to serialise session.json: {}", e))?;

        let temp = path.with_extension("json.tmp");
        fs::write(&temp, json).map_err(|e| format!("Failed to write session.json: {}", e))?;
        fs::rename(&temp, path).map_err(|e| {
            let _ = fs::remove_file(&temp);
            format!("Failed to replace session.json: {}", e)
        })
    }
}
