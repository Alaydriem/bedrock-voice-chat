pub mod ncryptflib;

pub mod auth;
pub mod encoding;
pub mod errors;
pub use serde::{Deserialize, Serialize};

pub mod consts;
pub mod request;
#[cfg(feature = "quic")]
pub mod rustls;
pub mod structs;
pub mod traits;
pub mod players;
pub mod game_data;

// Re-export error types
pub use errors::{
    CommunicationError, GameError, GenericCommunicationError, HytaleCommunicationError,
    MinecraftCommunicationError,
};

// Re-export s2n-quic when feature is enabled
#[cfg(feature = "quic")]
pub use s2n_quic;

// Re-export important types for easy access
pub use structs::player_source::PlayerSource;
pub use structs::recording::{RecordingPlayerData, SessionManifest};

// Re-export new player system types
pub use players::{GenericPlayer, HytalePlayer, MinecraftPlayer, PlayerEnum};
pub use game_data::{GameDataCollection, Dimension, HytaleDimension};
pub use traits::player_data::{PlayerData as PlayerDataTrait, SpatialPlayer};

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum Game {
    #[serde(rename = "minecraft")]
    Minecraft,
    #[serde(rename = "hytale")]
    Hytale,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Coordinate {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Default for Coordinate {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Orientation {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Player {
    pub name: String,
    pub dimension: Dimension,
    pub deafen: bool,
    pub coordinates: Coordinate,
    pub orientation: Orientation,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GameData {
    pub game: Option<Game>,
    pub players: Vec<Player>,
}
