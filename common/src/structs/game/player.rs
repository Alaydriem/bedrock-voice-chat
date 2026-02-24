use serde::{Deserialize, Serialize};

use crate::game_data::Dimension;
use super::coordinate::Coordinate;
use super::orientation::Orientation;

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
