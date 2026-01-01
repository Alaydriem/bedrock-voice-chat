use crate::{Coordinate, Game, Orientation};
use crate::game_data::Dimension;
use crate::traits::player_data::{PlayerData, SpatialPlayer};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MinecraftPlayer {
    pub name: String,
    pub coordinates: Coordinate,
    pub orientation: Orientation,
    pub dimension: Dimension,
    pub deafen: bool
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
    pub fn can_communicate_with(&self, other: &MinecraftPlayer, range: f32) -> bool {
        if !self.dimension.eq(&other.dimension) {
            return false;
        }

        let proximity = 1.73 * range;
        self.distance_to(other) <= proximity
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
        }
    }
}
