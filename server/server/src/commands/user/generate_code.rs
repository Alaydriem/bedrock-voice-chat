use clap::Parser;
use common::Game;
use common::request::admin::GenerateCodeRequest;

use super::super::Cli;
use crate::commands::admin_api_client::AdminApiClient;
use crate::commands::admin_api_error::AdminApiError;

#[derive(Debug, Parser, Clone)]
#[clap(author, version, about = "Generate a login code for a player", long_about = None)]
pub struct Config {
    /// The player's gamertag
    #[clap(short, long)]
    pub player: String,

    /// The game type (minecraft or hytale)
    #[clap(short, long, value_enum)]
    pub game: Game,

    /// How long the code is valid for, in seconds
    #[clap(short, long, default_value = "3600")]
    pub duration: u64,

    /// Whether the code is single-use (consumed on redemption). Pass
    /// `--ephemeral false` to mint a reusable code valid until expiry.
    #[clap(long, action = clap::ArgAction::Set, default_value_t = true)]
    pub ephemeral: bool,
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

        let req = GenerateCodeRequest {
            gamertag: self.player.clone(),
            game: self.game.clone(),
            duration: self.duration,
            ephemeral: self.ephemeral,
        };

        match client.generate_code(&req).await {
            Ok(resp) => {
                println!("Code: {}", resp.code);
                println!("Player: {} ({})", self.player, self.game);
                println!("Expires in: {}s", resp.expires_in_seconds);
            }
            Err(AdminApiError::NotFound) => eprintln!(
                "Player '{}' not found for game '{}'. Add the player first with `bvc user add`.",
                self.player, self.game
            ),
            Err(e) => {
                eprintln!("Failed to generate code: {}", e);
                std::process::exit(1);
            }
        }
    }
}
