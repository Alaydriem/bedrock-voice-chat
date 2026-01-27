use crate::errors::CommunicationError;
use crate::traits::player_data::{PlayerData, SpatialPlayer};
use crate::{Coordinate, Game, Orientation};
use serde::{Deserialize, Serialize};

/// Generic player implementation for games that don't need special spatial logic
/// Uses simple coordinate-based distance calculation without game-specific concepts
/// like dimensions, realms, zones, etc.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GenericPlayer {
    pub name: String,
    pub coordinates: Coordinate,
    pub orientation: Orientation,
    pub game: Game,
}

impl PlayerData for GenericPlayer {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_position(&self) -> &Coordinate {
        &self.coordinates
    }

    fn get_orientation(&self) -> &Orientation {
        &self.orientation
    }

    fn get_game(&self) -> Game {
        self.game.clone()
    }

    fn clone_box(&self) -> Box<dyn PlayerData> {
        Box::new(self.clone())
    }
}

impl SpatialPlayer for GenericPlayer {}

impl GenericPlayer {
    pub fn can_communicate_with(
        &self,
        other: &GenericPlayer,
        range: f32,
    ) -> Result<(), CommunicationError> {
        let proximity = 1.73 * range;
        let distance = self.distance_to(other);
        if distance > proximity {
            return Err(CommunicationError::OutOfRange {
                distance,
                max_range: proximity,
            });
        }
        Ok(())
    }
}
