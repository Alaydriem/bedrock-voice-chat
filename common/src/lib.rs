pub mod ncryptflib;

pub mod auth;
pub mod encoding;
pub mod errors;
pub use serde::{Deserialize, Serialize};

pub mod consts;
pub mod request;
pub mod response;
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

pub use reqwest;

// Re-export s2n-quic when feature is enabled
#[cfg(feature = "quic")]
pub use s2n_quic;
#[cfg(feature = "quic")]
pub use s2n_quic_core;

// Re-export cpal when audio feature is enabled
#[cfg(feature = "audio")]
pub use rodio::cpal;

// Re-export game types for backward compatibility
pub use structs::game::{Game, Coordinate, Orientation, Player, GameData};

// Re-export important types for easy access
pub use structs::players::PlayerSource;
pub use structs::recording::{RecordingPlayerData, SessionManifest};

// Re-export new player system types
pub use players::{GenericPlayer, HytalePlayer, MinecraftPlayer, PlayerEnum};
pub use game_data::{GameDataCollection, Dimension, HytaleDimension};
pub use traits::player_data::{PlayerData as PlayerDataTrait, SpatialPlayer};
