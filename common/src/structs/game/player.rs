use serde::{Deserialize, Serialize};

use super::coordinate::Coordinate;
use super::orientation::Orientation;
use crate::game_data::Dimension;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct Player {
    pub name: String,
    pub dimension: Dimension,
    pub deafen: bool,
    pub coordinates: Coordinate,
    pub orientation: Orientation,
    #[serde(default)]
    pub spectator: bool,
}
