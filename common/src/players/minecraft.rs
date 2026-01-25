use crate::errors::{CommunicationError, MinecraftCommunicationError};
use crate::game_data::Dimension;
use crate::traits::player_data::{PlayerData, SpatialPlayer};
use crate::{Coordinate, Game, Orientation};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MinecraftPlayer {
    pub name: String,
    pub coordinates: Coordinate,
    pub orientation: Orientation,
    pub dimension: Dimension,
    pub deafen: bool,
    #[serde(default)]
    pub spectator: bool,
}

impl PlayerData for MinecraftPlayer {
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
        Game::Minecraft
    }

    fn clone_box(&self) -> Box<dyn PlayerData> {
        Box::new(self.clone())
    }
}

impl SpatialPlayer for MinecraftPlayer {}

impl MinecraftPlayer {
    pub fn can_communicate_with(
        &self,
        other: &MinecraftPlayer,
        range: f32,
    ) -> Result<(), CommunicationError> {
        if !self.dimension.eq(&other.dimension) {
            return Err(CommunicationError::minecraft(
                MinecraftCommunicationError::DimensionMismatch {
                    sender: self.dimension.clone(),
                    recipient: other.dimension.clone(),
                },
            ));
        }

        // Spectator logic: spectators hear everyone, but non-spectators can't hear spectators
        if self.spectator && !other.spectator {
            return Err(CommunicationError::minecraft(
                MinecraftCommunicationError::SpectatorInaudible,
            ));
        }

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

impl From<crate::Player> for MinecraftPlayer {
    fn from(player: crate::Player) -> Self {
        Self {
            name: player.name,
            coordinates: player.coordinates,
            orientation: player.orientation,
            dimension: player.dimension,
            deafen: player.deafen,
            spectator: player.spectator,
        }
    }
}

impl From<MinecraftPlayer> for crate::Player {
    fn from(player: MinecraftPlayer) -> Self {
        Self {
            name: player.name,
            coordinates: player.coordinates,
            orientation: player.orientation,
            dimension: player.dimension,
            deafen: player.deafen,
            spectator: player.spectator,
        }
    }
}
