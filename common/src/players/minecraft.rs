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
    #[serde(default)]
    pub world_uuid: Option<String>,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_player(world_uuid: Option<&str>) -> MinecraftPlayer {
        MinecraftPlayer {
            name: "Player".to_string(),
            coordinates: Coordinate { x: 0.0, y: 0.0, z: 0.0 },
            orientation: Orientation { x: 0.0, y: 0.0 },
            dimension: Dimension::Overworld,
            deafen: false,
            spectator: false,
            world_uuid: world_uuid.map(String::from),
        }
    }

    #[test]
    fn world_uuid_mismatch_blocks_communication() {
        let a = make_player(Some("world-a"));
        let b = make_player(Some("world-b"));
        let err = a.can_communicate_with(&b, 100.0).unwrap_err();
        assert!(matches!(
            err,
            CommunicationError::Game(crate::errors::GameError::Minecraft(
                MinecraftCommunicationError::WorldMismatch { .. }
            ))
        ));
    }

    #[test]
    fn world_uuid_match_allows_communication() {
        let a = make_player(Some("world-a"));
        let b = make_player(Some("world-a"));
        assert!(a.can_communicate_with(&b, 100.0).is_ok());
    }

    #[test]
    fn world_uuid_none_none_allows_communication() {
        let a = make_player(None);
        let b = make_player(None);
        assert!(a.can_communicate_with(&b, 100.0).is_ok());
    }

    #[test]
    fn world_uuid_some_none_allows_communication() {
        let a = make_player(Some("world-a"));
        let b = make_player(None);
        assert!(a.can_communicate_with(&b, 100.0).is_ok());
    }

    #[test]
    fn world_uuid_none_some_allows_communication() {
        let a = make_player(None);
        let b = make_player(Some("world-a"));
        assert!(a.can_communicate_with(&b, 100.0).is_ok());
    }

    #[test]
    fn world_uuid_json_deserialization_without_field() {
        let json = r#"{
            "name": "Test",
            "coordinates": { "x": 0.0, "y": 0.0, "z": 0.0 },
            "orientation": { "x": 0.0, "y": 0.0 },
            "dimension": "overworld",
            "deafen": false
        }"#;
        let player: MinecraftPlayer = serde_json::from_str(json).unwrap();
        assert_eq!(player.world_uuid, None);
    }

    #[test]
    fn world_uuid_json_deserialization_with_field() {
        let json = r#"{
            "name": "Test",
            "coordinates": { "x": 0.0, "y": 0.0, "z": 0.0 },
            "orientation": { "x": 0.0, "y": 0.0 },
            "dimension": "overworld",
            "deafen": false,
            "world_uuid": "550e8400-e29b-41d4-a716-446655440000"
        }"#;
        let player: MinecraftPlayer = serde_json::from_str(json).unwrap();
        assert_eq!(player.world_uuid, Some("550e8400-e29b-41d4-a716-446655440000".to_string()));
    }
}
