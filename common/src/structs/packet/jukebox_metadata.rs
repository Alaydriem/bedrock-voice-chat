use serde::{Deserialize, Serialize};

use crate::game_data::Dimension;
use crate::structs::game::Coordinate;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct JukeboxMetadata {
    pub position: Coordinate,
    pub event_id: String,
    pub dimension: Dimension,
}

impl JukeboxMetadata {
    pub fn new(position: Coordinate, event_id: String, dimension: Dimension) -> Self {
        Self {
            position,
            event_id,
            dimension,
        }
    }
}
