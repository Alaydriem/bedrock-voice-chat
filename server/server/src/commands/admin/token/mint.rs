use clap::Parser;
use common::response::admin::MintedTokenResponse;

use bvc_server_lib::services::AccessTokenService;

use crate::commands::Cli;
use crate::commands::admin::AdminApiClient;

#[derive(Debug, Parser, Clone)]
#[clap(author, version, about = "Issue a game server access token", long_about = None)]
pub struct Config {
    /// Write directly to the database named by -c, with no running server and no admin
    /// identity
    #[clap(long)]
    pub local: bool,
}

impl Config {
    pub async fn run<'a>(&'a self, cfg: &Cli) {
        let minted = if self.local {
            self.mint_locally(cfg).await
        } else {
            self.mint_remotely(cfg).await
        };

        eprintln!(
            "Token {} issued. This is the only time it is shown; the server keeps only its \
             hash. Put it in your Addon or mod configuration now.",
            minted.id
        );
        println!("{}", minted.token);

        if self.local {
            eprintln!(
                "A running server begins accepting it within 15 seconds. No restart is needed."
            );
        }
    }

    async fn mint_locally(&self, cfg: &Cli) -> MintedTokenResponse {
        let db = match cfg.config.create_database_connection().await {
            Ok(conn) => conn,
            Err(e) => {
                eprintln!("Failed to connect to database: {}", e);
                std::process::exit(1);
            }
        };

        match AccessTokenService::mint_in(&db).await {
            Ok(minted) => minted,
            Err(e) => {
                eprintln!("Failed to mint a token: {}", e);
                std::process::exit(1);
            }
        }
    }

    async fn mint_remotely(&self, cfg: &Cli) -> MintedTokenResponse {
        let client = match AdminApiClient::from_active_identity(cfg.identity.as_deref()) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        };

        match client.mint_access_token().await {
            Ok(minted) => minted,
            Err(e) => {
                eprintln!("Failed to mint a token: {}", e);
                std::process::exit(1);
            }
        }
    }
}
