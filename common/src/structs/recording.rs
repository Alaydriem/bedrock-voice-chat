use serde::{Deserialize, Serialize};
use ts_rs::TS;
use crate::structs::audio::PlayerGainSettings;
use crate::structs::packet::{PacketOwner, AudioFramePacket};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecordingPlayerData {
    pub name: String,
    pub client_id: Option<Vec<u8>>,
    pub player_data: Option<crate::PlayerEnum>,
    pub spatial: Option<bool>,
    pub gain_settings: Option<PlayerGainSettings>,
}

impl RecordingPlayerData {
    /// Create PlayerData from packet owner and audio frame data
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

    /// Create PlayerData from Player cache entry (for listener)
    pub fn from_player(
        player: &crate::Player,
        player_name: String,
        gain_settings: Option<PlayerGainSettings>,
    ) -> Self {
        // Convert legacy Player to MinecraftPlayer, then to PlayerEnum
        let mc_player = crate::players::MinecraftPlayer::from(player.clone());
        Self {
            name: player_name,
            client_id: None,
            player_data: Some(crate::PlayerEnum::Minecraft(mc_player)),
            spatial: None,
            gain_settings,
        }
    }

    /// Create RecordingPlayerData from PlayerEnum cache entry (for listener)
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

    /// Create unknown/fallback PlayerData
    pub fn unknown() -> Self {
        Self {
            name: "unknown".to_string(),
            client_id: None,
            player_data: None,
            spatial: None,
            gain_settings: None,
        }
    }

    /// Create PlayerData for current player input (no position data yet)
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

    /// Extract metadata (strip name/client_id) for WAL header storage
    pub fn to_metadata(&self) -> PlayerMetadata {
        PlayerMetadata {
            player_data: self.player_data.clone(),
            spatial: self.spatial,
            gain_settings: self.gain_settings.clone(),
        }
    }
}

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
    ) -> RecordingPlayerData {
        RecordingPlayerData {
            name,
            client_id,
            player_data: self.player_data,
            spatial: self.spatial,
            gain_settings: self.gain_settings,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct SessionManifest {
    pub session_id: String,
    pub start_timestamp: u64,
    pub end_timestamp: Option<u64>,
    pub duration_ms: Option<u64>,
    pub emitter_player: String,
    pub participants: Vec<String>,
    pub created_at: String,
}

/// Concrete header type for input recording WAL entries
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InputRecordingHeader {
    pub sample_rate: u32,
    pub channels: u16,
    pub relative_timestamp_ms: Option<u64>,
    pub emitter_metadata: PlayerMetadata,
}

/// Concrete header type for output recording WAL entries
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OutputRecordingHeader {
    pub sample_rate: u32,
    pub channels: u16,
    pub relative_timestamp_ms: u64,
    pub emitter_metadata: PlayerMetadata,
    pub listener_metadata: PlayerMetadata,
    pub is_spatial: bool,
}

/// Union type for all recording headers to enable type-safe decoding
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RecordingHeader {
    Input(InputRecordingHeader),
    Output(OutputRecordingHeader),
}

impl RecordingHeader {
    /// Decode a header from postcard bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, postcard::Error> {
        postcard::from_bytes(bytes)
    }

    /// Encode a header to postcard bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, postcard::Error> {
        postcard::to_allocvec(self)
    }

    /// Get the sample rate from any header type
    pub fn sample_rate(&self) -> u32 {
        match self {
            RecordingHeader::Input(header) => header.sample_rate,
            RecordingHeader::Output(header) => header.sample_rate,
        }
    }

    /// Get the channel count from any header type
    pub fn channels(&self) -> u16 {
        match self {
            RecordingHeader::Input(header) => header.channels,
            RecordingHeader::Output(header) => header.channels,
        }
    }

    /// Check if this is a spatial audio recording
    pub fn is_spatial(&self) -> bool {
        match self {
            RecordingHeader::Input(_) => false, // Input is never spatial
            RecordingHeader::Output(header) => header.is_spatial,
        }
    }
}

impl From<&crate::Player> for RecordingPlayerData {
    fn from(player: &crate::Player) -> Self {
        // Convert legacy Player to MinecraftPlayer, then to PlayerEnum
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