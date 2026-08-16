use clap::Parser;

use crate::commands::Cli;
use crate::commands::admin_api_client::AdminApiClient;

#[derive(Debug, Parser, Clone)]
#[clap(author, version, about = "List the relay worlds this server is hosting", long_about = None)]
pub struct Config {}

impl Config {
    pub async fn run<'a>(&'a self, cfg: &Cli) {
        let client = match AdminApiClient::from_active_identity(cfg.identity.as_deref()) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        };

        match client.relay_worlds().await {
            Ok(resp) => {
                if resp.worlds.is_empty() {
                    println!(
                        "No player is currently in a relay world. A world id appears here \
                         once a player joins one."
                    );
                    return;
                }

                println!("{:<40}{}", "relay world", "players");
                for world in resp.worlds {
                    println!("{:<40}{}", world.world, world.players);
                }
            }
            Err(e) => {
                eprintln!("Failed to list relay worlds: {}", e);
                std::process::exit(1);
            }
        }
    }
}
