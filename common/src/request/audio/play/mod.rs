pub mod hytale;
pub mod minecraft;

pub use hytale::HytaleAudioContext;
pub use minecraft::MinecraftAudioContext;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct AudioPlayRequest {
    pub audio_file_id: String,
    pub game: GameAudioContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[serde(tag = "game")]
pub enum GameAudioContext {
    #[serde(rename = "minecraft")]
    Minecraft(MinecraftAudioContext),
    #[serde(rename = "hytale")]
    Hytale(HytaleAudioContext),
}
