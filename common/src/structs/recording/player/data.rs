use serde::{Deserialize, Serialize};

use crate::structs::audio::PlayerGainSettings;
use crate::structs::packet::{PacketOwner, AudioFramePacket};
use super::metadata::PlayerMetadata;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecordingPlayerData {
    pub name: String,
    pub client_id: Option<Vec<u8>>,
    pub player_data: Option<crate::PlayerEnum>,
    pub spatial: Option<bool>,
    pub gain_settings: Option<PlayerGainSettings>,
}

impl RecordingPlayerData {
    pub fn from_packet_owner(
        owner: &PacketOwner,
        audio_data: &AudioFramePacket,
        gain_settings: Option<PlayerGainSettings>,
    ) -> Self {
        Self {
            name: owner.name.clone(),
            client_id: Some(owner.client_id.clone()),
            player_data: audio_data.sender.clone(),
            spatial: audio_data.spatial,
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
            client_id: None,
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
            client_id: None,
            player_data: Some(player.clone()),
            spatial: None,
            gain_settings,
        }
    }

    pub fn unknown() -> Self {
        Self {
            name: "unknown".to_string(),
            client_id: None,
            player_data: None,
            spatial: None,
            gain_settings: None,
        }
    }

    pub fn for_input(
        player_name: String,
        gain_settings: Option<PlayerGainSettings>,
    ) -> Self {
        Self {
            name: player_name,
            client_id: None,
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
            client_id: None,
            player_data: Some(crate::PlayerEnum::Minecraft(mc_player)),
            spatial: None,
            gain_settings: None,
        }
    }
}
