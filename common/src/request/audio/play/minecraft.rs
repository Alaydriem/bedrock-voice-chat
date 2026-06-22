use serde::{Deserialize, Serialize};

use crate::Coordinate;
use crate::game_data::Dimension;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct MinecraftAudioContext {
    pub coordinates: Coordinate,
    pub dimension: Dimension,
    pub world_uuid: String,
    #[serde(default)]
    pub relay_world_uuid: Option<String>,
}
