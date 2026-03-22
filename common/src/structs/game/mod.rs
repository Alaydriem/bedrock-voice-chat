pub mod coordinate;
pub mod data;
pub mod identity;
pub mod orientation;
pub mod player;

pub use coordinate::Coordinate;
pub use data::GameData;
pub use identity::UploaderIdentity;
pub use orientation::Orientation;
pub use player::Player;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash, ts_rs::TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
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
