use serde::{Deserialize, Serialize};

use crate::game_data::Dimension;
use crate::Coordinate;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct MinecraftAudioContext {
    pub coordinates: Coordinate,
    pub dimension: Dimension,
    pub world_uuid: String,
}
