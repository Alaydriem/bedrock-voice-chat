pub mod minecraft;

use std::sync::LazyLock;
use std::time::Duration;

use crate::Game;
use crate::logging::LogThrottle;
use crate::players::{MinecraftPlayer, PlayerEnum};
use serde::{Deserialize, Deserializer, Serialize};

pub use minecraft::{Dimension, DimensionHash};

// A mod sends its batch every tick, so one bad record would otherwise log the same line
// many times a second.
static REJECTED_RECORD_LOG: LazyLock<LogThrottle> =
    LazyLock::new(|| LogThrottle::new(Duration::from_secs(60)));

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
                        if let Some(suppressed) = REJECTED_RECORD_LOG.should_log() {
                            // serde embeds the offending value in its message, which a mod
                            // can make as large as the request body.
                            let error: String = e.to_string().chars().take(128).collect();
                            crate::curia::warn!("dropped a player record that did not parse", {
                                "error": error,
                                "suppressed": suppressed,
                            });
                        }
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
