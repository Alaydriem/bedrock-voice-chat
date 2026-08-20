use clap::Parser;
use common::Game;
use common::request::admin::BanishUserRequest;

use crate::commands::Cli;
use crate::commands::admin::AdminApiClient;
use crate::commands::admin::AdminApiError;

#[derive(Debug, Parser, Clone)]
#[clap(author, version, about = "Banish or unbanish a player", long_about = None)]
pub struct Config {
    /// The player's gamertag
    #[clap(short, long)]
    pub player: String,

    /// The game type (minecraft)
    #[clap(short, long, value_enum)]
    pub game: Game,

    /// Set to true to banish, false to unbanish
    #[clap(short, long, action = clap::ArgAction::Set, default_value_t = true)]
    pub banish: bool,
}

impl Config {
    pub async fn run<'a>(&'a self, cfg: &Cli) {
        let client = match AdminApiClient::from_active_identity(cfg.identity.as_deref()) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        };

        let req = BanishUserRequest {
            gamertag: self.player.clone(),
            game: self.game.clone(),
            banish: self.banish,
        };

        match client.banish_user(&req).await {
            Ok(_) => {
                let action = if self.banish {
                    "banished"
                } else {
                    "unbanished"
                };
                println!(
                    "Successfully {} player '{}' for game '{}'",
                    action, self.player, self.game
                );
            }
            Err(AdminApiError::NotFound) => eprintln!(
                "Player '{}' not found for game '{}'",
                self.player, self.game
            ),
            Err(e) => {
                eprintln!("Failed to update player: {}", e);
                std::process::exit(1);
            }
        }
    }
}
