use serde::{Deserialize, Serialize};

use crate::game_data::Dimension;
use crate::Coordinate;

/// Request to start audio playback at a location.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioPlayRequest {
    pub audio_file_id: String,
    pub coordinates: Coordinate,
    pub dimension: Dimension,
    pub world_uuid: String,
}
