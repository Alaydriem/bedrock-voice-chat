use std::path::Path;

use common::structs::recording::SessionManifest;

#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub session_id: String,
    pub start_timestamp: u64,
    pub player_name: String,
    pub duration_ms: Option<u64>,
}

impl SessionInfo {
    pub fn load(session_path: &Path) -> Result<Self, anyhow::Error> {
        let session_json_path = session_path.join("session.json");
        let manifest: SessionManifest =
            serde_json::from_str(&std::fs::read_to_string(session_json_path)?)?;

        Ok(Self {
            session_id: manifest.session_id,
            start_timestamp: manifest.start_timestamp,
            player_name: manifest.emitter_player,
            duration_ms: manifest.duration_ms,
        })
    }
}
