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

use std::fmt;

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

// Re-export important types for easy access
pub use structs::player_source::PlayerSource;
pub use structs::recording::{RecordingPlayerData, SessionManifest};

// Re-export new player system types
pub use players::{GenericPlayer, HytalePlayer, MinecraftPlayer, PlayerEnum};
pub use game_data::{GameDataCollection, Dimension, HytaleDimension};
pub use traits::player_data::{PlayerData as PlayerDataTrait, SpatialPlayer};

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash, ts_rs::TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
#[cfg_attr(feature = "server", derive(sea_orm::EnumIter, sea_orm::DeriveActiveEnum, clap::ValueEnum))]
#[cfg_attr(feature = "server", sea_orm(rs_type = "String", db_type = "Text"))]
pub enum Game {
    #[serde(rename = "minecraft")]
    #[cfg_attr(feature = "server", sea_orm(string_value = "minecraft"))]
    Minecraft,
    #[serde(rename = "hytale")]
    #[cfg_attr(feature = "server", sea_orm(string_value = "hytale"))]
    Hytale,
}

impl Game {
    pub fn as_str(&self) -> &'static str {
        match self {
            Game::Minecraft => "minecraft",
            Game::Hytale => "hytale",
        }
    }
}

impl fmt::Display for Game {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(feature = "server")]
impl<'r> rocket::request::FromParam<'r> for Game {
    type Error = &'r str;

    fn from_param(param: &'r str) -> Result<Self, Self::Error> {
        match param {
            "minecraft" => Ok(Game::Minecraft),
            "hytale" => Ok(Game::Hytale),
            _ => Err(param),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
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
    #[serde(default)]
    pub spectator: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GameData {
    pub game: Option<Game>,
    pub players: Vec<Player>,
}
