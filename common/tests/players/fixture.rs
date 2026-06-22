use common::game_data::Dimension;
use common::{Coordinate, MinecraftPlayer, Orientation};

pub struct PlayerFixture;

impl PlayerFixture {
    pub fn make(world_uuid: Option<&str>) -> MinecraftPlayer {
        MinecraftPlayer {
            name: "Player".to_string(),
            coordinates: Coordinate {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            orientation: Orientation { x: 0.0, y: 0.0 },
            dimension: Dimension::Overworld,
            deafen: false,
            spectator: false,
            world_uuid: world_uuid.map(String::from),
            alternative_identity: None,
            player_uuid: None,
            relay_world_uuid: None,
        }
    }

    pub fn make_with_relay(
        world_uuid: Option<&str>,
        relay_world_uuid: Option<&str>,
    ) -> MinecraftPlayer {
        MinecraftPlayer {
            relay_world_uuid: relay_world_uuid.map(String::from),
            ..Self::make(world_uuid)
        }
    }
}
