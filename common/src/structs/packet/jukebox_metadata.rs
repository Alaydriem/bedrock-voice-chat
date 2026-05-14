use serde::{Deserialize, Serialize};

use crate::structs::game::Coordinate;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct JukeboxMetadata {
    pub position: Coordinate,
    pub event_id: String,
}

impl JukeboxMetadata {
    pub fn new(position: Coordinate, event_id: String) -> Self {
        Self { position, event_id }
    }
}
