use clap::Parser;
use common::Game;
use common::request::admin::CreateUserRequest;

use super::super::Cli;
use crate::commands::admin::AdminApiClient;
use crate::commands::admin::AdminApiError;

#[derive(Debug, Parser, Clone)]
#[clap(author, version, about = "Add a player to the server", long_about = None)]
pub struct Config {
    /// The player's gamertag
    #[clap(short, long)]
    pub player: String,

    /// The game type (minecraft)
    #[clap(short, long, value_enum)]
    pub game: Game,
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

        let req = CreateUserRequest {
            gamertag: self.player.clone(),
            game: self.game.clone(),
        };

        match client.create_user(&req).await {
            Ok(_) => println!(
                "Successfully added player '{}' for game '{}'",
                self.player, self.game
            ),
            Err(AdminApiError::Conflict) => println!(
                "Player '{}' already exists for game '{}'",
                self.player, self.game
            ),
            Err(e) => {
                eprintln!("Failed to add player: {}", e);
                std::process::exit(1);
            }
        }
    }
}
