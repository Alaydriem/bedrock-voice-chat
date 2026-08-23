use clap::Parser;
use common::Game;
use common::structs::permission::PermissionEffect;

use crate::commands::Cli;
use crate::commands::admin::AdminApiClient;
use crate::commands::admin::AdminApiError;

#[derive(Debug, Parser, Clone)]
#[clap(author, version, about = "List permission overrides for a player", long_about = None)]
pub struct Config {
    #[clap(short, long)]
    pub player: String,
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

        match client.list_permissions(&self.player, &self.game).await {
            Ok(resp) => {
                if resp.entries.is_empty() {
                    println!(
                        "No permission overrides for player '{}' (using config defaults)",
                        self.player
                    );
                } else {
                    println!("Permission overrides for player '{}':", self.player);
                    for entry in resp.entries {
                        let effect_str = match entry.effect {
                            PermissionEffect::Allow => "allow",
                            PermissionEffect::Deny => "deny",
                        };
                        println!("  {} = {}", entry.permission, effect_str);
                    }
                }
            }
            Err(AdminApiError::NotFound) => eprintln!(
                "Player '{}' not found for game '{}'",
                self.player, self.game
            ),
            Err(e) => {
                eprintln!("Failed to query permissions: {}", e);
                std::process::exit(1);
            }
        }
    }
}
