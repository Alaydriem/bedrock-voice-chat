use serde::{Deserialize, Serialize};

use super::Game;
use super::player::Player;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct GameData {
    pub game: Option<Game>,
    pub players: Vec<Player>,
}
