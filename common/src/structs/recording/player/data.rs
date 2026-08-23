use serde::{Deserialize, Serialize};

use super::metadata::PlayerMetadata;
use crate::structs::audio::PlayerGainSettings;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecordingPlayerData {
    /// The canonical identity, `game:gamertag`.
    pub name: String,
    /// Which of that player's devices this track came from, so a player recorded from two
    /// devices renders as two tracks rather than one interleaved mess. Absent for a track the
    /// server injected and for the local input track, neither of which has a connection.
    pub device: Option<u64>,
    pub player_data: Option<crate::PlayerEnum>,
    pub spatial: Option<bool>,
    pub gain_settings: Option<PlayerGainSettings>,
}

impl RecordingPlayerData {
    /// The emitter of one recorded track.
    ///
    /// `name` is the speaker as the caller resolved it: a player's canonical identity
    /// rendered, or the service name for injected audio. Resolved by the caller rather than
    /// read off the sender, because a reduced sender names only a device.
    ///
    /// `player_data` is composed by the caller too. The recorded header has always held a whole
    /// player and the renderer reads a position and a deafened flag back out of it, but the
    /// frame carries only those two facts now — so the caller builds one rather than this
    /// reading a wire type.
    pub fn from_speaker(
        name: String,
        device: Option<u64>,
        player_data: Option<crate::PlayerEnum>,
        spatial: Option<bool>,
        gain_settings: Option<PlayerGainSettings>,
    ) -> Self {
        Self {
            name,
            device,
            player_data,
            spatial,
            gain_settings,
        }
    }

    pub fn from_player(
        player: &crate::Player,
        player_name: String,
        gain_settings: Option<PlayerGainSettings>,
    ) -> Self {
        let mc_player = crate::players::MinecraftPlayer::from(player.clone());
        Self {
            name: player_name,
            device: None,
            player_data: Some(crate::PlayerEnum::Minecraft(mc_player)),
            spatial: None,
            gain_settings,
        }
    }

    pub fn from_player_enum(
        player: &crate::PlayerEnum,
        player_name: String,
        gain_settings: Option<PlayerGainSettings>,
    ) -> Self {
        Self {
            name: player_name,
            device: None,
            player_data: Some(player.clone()),
            spatial: None,
            gain_settings,
        }
    }

    pub fn unknown() -> Self {
        Self {
            name: "unknown".to_string(),
            device: None,
            player_data: None,
            spatial: None,
            gain_settings: None,
        }
    }

    pub fn for_input(player_name: String, gain_settings: Option<PlayerGainSettings>) -> Self {
        Self {
            name: player_name,
            device: None,
            player_data: None,
            spatial: None,
            gain_settings,
        }
    }

    pub fn to_metadata(&self) -> PlayerMetadata {
        PlayerMetadata {
            player_data: self.player_data.clone(),
            spatial: self.spatial,
            gain_settings: self.gain_settings.clone(),
        }
    }
}

impl From<&crate::Player> for RecordingPlayerData {
    fn from(player: &crate::Player) -> Self {
        let mc_player = crate::players::MinecraftPlayer::from(player.clone());
        Self {
            name: player.name.clone(),
            device: None,
            player_data: Some(crate::PlayerEnum::Minecraft(mc_player)),
            spatial: None,
            gain_settings: None,
        }
    }
}
