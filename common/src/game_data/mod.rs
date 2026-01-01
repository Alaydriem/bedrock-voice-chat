pub mod minecraft;

use crate::players::{MinecraftPlayer, GenericPlayer, PlayerEnum};
use crate::Game;
use serde::{Deserialize, Deserializer, Serialize};

pub use minecraft::Dimension;

/// Container for game session data with heterogeneous player types
#[derive(Clone, Debug, Serialize)]
pub struct GameDataCollection {
    pub game: Option<Game>,
    pub players: Vec<PlayerEnum>,
}

impl<'de> Deserialize<'de> for GameDataCollection {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;

        // Helper struct for deserializing JSON from legacy clients
        #[derive(Deserialize)]
        struct LegacyFormat {
            game: Option<Game>,
            players: Vec<serde_json::Value>,
        }

        let legacy = LegacyFormat::deserialize(deserializer)?;
        let game_type = legacy.game.clone().unwrap_or(Game::Minecraft);

        // Convert players based on the game type
        let players: Result<Vec<PlayerEnum>, _> = legacy
            .players
            .into_iter()
            .map(|value| {
                match game_type {
                    Game::Minecraft => {
                        serde_json::from_value::<MinecraftPlayer>(value)
                            .map(PlayerEnum::Minecraft)
                            .map_err(D::Error::custom)
                    }
                    Game::Hytale => {
                        serde_json::from_value::<GenericPlayer>(value)
                            .map(PlayerEnum::Generic)
                            .map_err(D::Error::custom)
                    }
                }
            })
            .collect();

        Ok(GameDataCollection {
            game: legacy.game,
            players: players?,
        })
    }
}

impl GameDataCollection {
    pub fn new(game: Option<Game>) -> Self {
        Self {
            game,
            players: Vec::new(),
        }
    }

    /// Get strongly-typed access to players
    pub fn get_players(&self) -> &[PlayerEnum] {
        &self.players
    }

    /// Add a player to the collection
    pub fn add_player(&mut self, player: PlayerEnum) {
        self.players.push(player);
    }

    /// Get the number of players
    pub fn player_count(&self) -> usize {
        self.players.len()
    }
}
