mod bounded_log_text;
pub mod minecraft;
mod rejected_record_log;

pub use bounded_log_text::BoundedLogText;

use crate::Game;
use crate::players::{MinecraftPlayer, PlayerEnum};
use serde::{Deserialize, Deserializer, Serialize};

pub use minecraft::{Dimension, DimensionHash};
use rejected_record_log::RejectedRecordLog;

#[derive(Clone, Debug, Serialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct GameDataCollection {
    pub game: Option<Game>,
    pub players: Vec<PlayerEnum>,
}

impl<'de> Deserialize<'de> for GameDataCollection {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Helper struct for deserializing JSON from legacy clients
        #[derive(Deserialize)]
        struct LegacyFormat {
            game: Option<Game>,
            players: Vec<serde_json::Value>,
        }

        let legacy = LegacyFormat::deserialize(deserializer)?;
        let game_type = legacy.game.clone().unwrap_or(Game::Minecraft);

        // Each player parses on its own: one bad record drops that player, not the batch,
        // which would leave every position on the server stale for as long as it repeats.
        let players: Vec<PlayerEnum> = legacy
            .players
            .into_iter()
            .filter_map(|value| match game_type {
                Game::Minecraft => match serde_json::from_value::<MinecraftPlayer>(value) {
                    Ok(player) => Some(PlayerEnum::Minecraft(player)),
                    Err(e) => {
                        RejectedRecordLog::observe(&e.to_string());
                        None
                    }
                },
            })
            .collect();

        Ok(GameDataCollection {
            game: legacy.game,
            players,
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
