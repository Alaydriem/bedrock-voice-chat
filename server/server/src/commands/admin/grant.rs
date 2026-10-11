use clap::Parser;
use common::Game;
use common::structs::permission::AdminAction;

use super::AdminChange;
use crate::commands::Cli;

#[derive(Debug, Parser, Clone)]
#[clap(author, version, about = "Grant the `admin` permission to a player, registering them if needed. Runs locally against the DB.", long_about = None)]
pub struct Config {
    /// Player gamertag, matched exactly
    #[clap(short = 'p', long)]
    pub player: String,

    /// Game (minecraft)
    #[clap(short, long, value_enum, default_value_t = Game::Minecraft)]
    pub game: Game,
}

impl Config {
    pub async fn run(&self, cfg: &Cli) {
        AdminChange::run(cfg, &self.player, &self.game, AdminAction::Grant).await;
    }
}
