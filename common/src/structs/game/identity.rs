use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
#[serde(tag = "game")]
pub enum UploaderIdentity {
    #[serde(rename = "minecraft")]
    Minecraft { gamertag: String },
    #[serde(rename = "hytale")]
    Hytale { gamertag: String },
}

impl UploaderIdentity {
    pub fn from_game_str(game: &str, gamertag: String) -> Self {
        match game {
            "hytale" => UploaderIdentity::Hytale { gamertag },
            _ => UploaderIdentity::Minecraft { gamertag },
        }
    }

    pub fn gamertag(&self) -> &str {
        match self {
            UploaderIdentity::Minecraft { gamertag } => gamertag,
            UploaderIdentity::Hytale { gamertag } => gamertag,
        }
    }
}
