use clap::Parser;
use common::request::admin::ClearPermissionRequest;
use common::Game;

use crate::commands::admin_api_client::{AdminApiClient, AdminApiError};
use crate::commands::Cli;

#[derive(Debug, Parser, Clone)]
#[clap(author, version, about = "Clear a permission override (fall back to config default)", long_about = None)]
pub struct Config {
    #[clap(short, long)]
    pub player: String,
    #[clap(short, long, value_enum)]
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

        let req = ClearPermissionRequest {
            gamertag: self.player.clone(),
            game: self.game.clone(),
            permission: self.permission.clone(),
        };

        match client.clear_permission(&req).await {
            Ok(_) => println!(
                "Cleared permission override '{}' for player '{}' (will use config default)",
                self.permission, self.player
            ),
            Err(AdminApiError::NotFound) => println!(
                "No override found for permission '{}' on player '{}'",
                self.permission, self.player
            ),
            Err(e) => {
                eprintln!("Failed to delete permission override: {}", e);
                std::process::exit(1);
            }
        }
    }
}
