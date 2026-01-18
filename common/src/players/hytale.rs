//! Hytale-specific player implementation

use crate::errors::{CommunicationError, HytaleCommunicationError};
use crate::game_data::hytale::Dimension;
use crate::traits::player_data::{PlayerData, SpatialPlayer};
use crate::{Coordinate, Game, Orientation};
use serde::{Deserialize, Serialize};

/// Hytale player with world and dimension-based communication rules
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct HytalePlayer {
    pub name: String,
    pub coordinates: Coordinate,
    pub orientation: Orientation,
    #[serde(default)]
    pub world_uuid: Option<String>,
    #[serde(default)]
    pub dimension: Dimension,
    #[serde(default)]
    pub deafen: bool,
}

impl PlayerData for HytalePlayer {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_position(&self) -> &Coordinate {
        &self.coordinates
    }

    fn get_orientation(&self) -> &Orientation {
        &self.orientation
    }

    fn is_deafened(&self) -> bool {
        self.deafen
    }

    fn get_game(&self) -> Game {
        Game::Hytale
    }

    fn clone_box(&self) -> Box<dyn PlayerData> {
        Box::new(self.clone())
    }
}

impl SpatialPlayer for HytalePlayer {}

impl HytalePlayer {
    pub fn can_communicate_with(
        &self,
        other: &HytalePlayer,
        range: f32,
    ) -> Result<(), CommunicationError> {
        match (&self.world_uuid, &other.world_uuid) {
            (Some(self_world), Some(other_world)) if self_world != other_world => {
                return Err(CommunicationError::hytale(
                    HytaleCommunicationError::WorldMismatch {
                        sender_world: self_world.clone(),
                        recipient_world: other_world.clone(),
                    },
                ));
            }
            _ => {}
        }

        if !self.dimension.eq(&other.dimension) {
            return Err(CommunicationError::hytale(
                HytaleCommunicationError::DimensionMismatch {
                    sender: self.dimension.clone(),
                    recipient: other.dimension.clone(),
                },
            ));
        }

        let distance = self.distance_to(other);
        if distance > range {
            return Err(CommunicationError::OutOfRange {
                distance,
                max_range: range,
            });
        }

        Ok(())
    }
}
