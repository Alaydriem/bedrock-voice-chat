use clap::Parser;
use common::Game;
use common::request::admin::SetPermissionRequest;
use common::structs::permission::PermissionEffect;

use crate::commands::Cli;
use crate::commands::admin::AdminApiClient;
use crate::commands::admin::AdminApiError;

#[derive(Debug, Parser, Clone)]
#[clap(author, version, about = "Explicitly deny a permission for a player", long_about = None)]
pub struct Config {
    #[clap(short, long)]
    pub player: String,
    #[clap(short, long, value_enum, default_value_t = Game::Minecraft)]
    pub game: Game,
    #[clap(long)]
    pub permission: String,
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

        let req = SetPermissionRequest {
            gamertag: self.player.clone(),
            game: self.game.clone(),
            permission: self.permission.clone(),
            effect: PermissionEffect::Deny,
        };

        match client.set_permission(&req).await {
            Ok(_) => println!(
                "Denied permission '{}' for player '{}'",
                self.permission, self.player
            ),
            Err(AdminApiError::NotFound) => eprintln!(
                "Player '{}' not found for game '{}'",
                self.player, self.game
            ),
            Err(AdminApiError::BadRequest(_)) => {
                eprintln!("Unknown permission: '{}'", self.permission)
            }
            Err(e) => {
                eprintln!("Failed to update permission: {}", e);
                std::process::exit(1);
            }
        }
    }
}
