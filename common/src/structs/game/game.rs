use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
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

impl fmt::Display for Game {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
