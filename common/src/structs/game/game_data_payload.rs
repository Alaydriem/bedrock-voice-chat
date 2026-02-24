use serde::{Deserialize, Serialize};

use super::game::Game;
use super::player::Player;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GameData {
    pub game: Option<Game>,
    pub players: Vec<Player>,
}
