use serde::{Deserialize, Serialize};

use crate::structs::audio::PlayerGainSettings;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerMetadata {
    pub player_data: Option<crate::PlayerEnum>,
    pub spatial: Option<bool>,
    pub gain_settings: Option<PlayerGainSettings>,
}

impl PlayerMetadata {
    pub fn with_identity(
        self,
        name: String,
        device: Option<u64>,
    ) -> super::RecordingPlayerData {
        super::RecordingPlayerData {
            name,
            device,
            player_data: self.player_data,
            spatial: self.spatial,
            gain_settings: self.gain_settings,
        }
    }
}
