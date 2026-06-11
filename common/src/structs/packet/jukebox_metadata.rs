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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_data::Dimension;

    #[test]
    fn new_carries_dimension() {
        let m = JukeboxMetadata::new(
            Coordinate { x: 1.0, y: 2.0, z: 3.0 },
            "evt-1".to_string(),
            Dimension::TheNether,
        );
        assert_eq!(m.dimension, Dimension::TheNether);
        assert_eq!(m.event_id, "evt-1");
    }
}
