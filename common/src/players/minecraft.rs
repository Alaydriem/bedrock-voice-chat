use crate::errors::{CommunicationError, MinecraftCommunicationError};
use crate::game_data::Dimension;
use crate::traits::player_data::{PlayerData, SpatialPlayer};
use crate::{Coordinate, Game, Orientation};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct MinecraftPlayer {
    pub name: String,
    pub coordinates: Coordinate,
    pub orientation: Orientation,
    pub dimension: Dimension,
    pub deafen: bool,
    #[serde(default)]
    pub spectator: bool,
    #[serde(default)]
    pub world_uuid: Option<String>,
    #[serde(default)]
    pub alternative_identity: Option<String>,
    #[serde(default)]
    pub player_uuid: Option<String>,
    #[serde(default)]
    pub relay_world_uuid: Option<String>,
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
        match (&self.relay_world_uuid, &other.relay_world_uuid) {
            (Some(self_rw), Some(other_rw)) if self_rw != other_rw => {
                return Err(CommunicationError::minecraft(
                    MinecraftCommunicationError::WorldMismatch {
                        sender_world: self_rw.clone(),
                        recipient_world: other_rw.clone(),
                    },
                ));
            }
            _ => {}
        }

        match (&self.world_uuid, &other.world_uuid) {
            (Some(self_world), Some(other_world)) if self_world != other_world => {
                return Err(CommunicationError::minecraft(
                    MinecraftCommunicationError::WorldMismatch {
                        sender_world: self_world.clone(),
                        recipient_world: other_world.clone(),
                    },
                ));
            }
            _ => {}
        }

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
            world_uuid: None,
            alternative_identity: None,
            player_uuid: None,
            relay_world_uuid: None,
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
