use serde::{Deserialize, Serialize};

use crate::structs::audio::PlayerGainSettings;

/// Lightweight metadata without identity fields (for WAL headers)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerMetadata {
    pub player_data: Option<crate::PlayerEnum>,
    pub spatial: Option<bool>,
    pub gain_settings: Option<PlayerGainSettings>,
}

impl PlayerMetadata {
    /// Reconstitute full RecordingPlayerData by adding back identity
    pub fn with_identity(
        self,
        name: String,
        client_id: Option<Vec<u8>>,
    ) -> super::RecordingPlayerData {
        super::RecordingPlayerData {
            name,
            client_id,
            player_data: self.player_data,
            spatial: self.spatial,
            gain_settings: self.gain_settings,
        }
    }
}
