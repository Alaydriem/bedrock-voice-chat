pub mod resolver;
pub mod secrets;
pub mod store;

pub use resolver::IdentityResolver;
pub use store::{Identity, IdentityMetadata, IdentityStore, IdentitySummary};

use anyhow::anyhow;
use common::Game;

#[derive(Debug, Clone)]
pub struct IdentitySlot {
    pub gamertag: String,
    pub game: Game,
}

impl IdentitySlot {
    pub fn new(gamertag: impl Into<String>, game: Game) -> Self {
        Self {
            gamertag: gamertag.into(),
            game,
        }
    }

    pub fn parse(value: &str) -> Result<Self, anyhow::Error> {
        let (gamertag, game) = value
            .split_once(':')
            .ok_or_else(|| anyhow!("identity must be of the form '<gamertag>:<game>'"))?;
        let game = match game.to_lowercase().as_str() {
            "minecraft" => Game::Minecraft,
            "hytale" => Game::Hytale,
            other => return Err(anyhow!("unknown game '{}'", other)),
        };
        Ok(Self {
            gamertag: gamertag.to_string(),
            game,
        })
    }

    pub fn key(&self) -> String {
        format!("{}-{}", self.gamertag, self.game.as_str())
    }
}
