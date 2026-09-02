use clap::Parser;
use common::response::admin::MintedTokenResponse;

use bvc_server_lib::services::AccessTokenService;

use crate::commands::Cli;
use crate::commands::admin::AdminApiClient;

#[derive(Debug, Parser, Clone)]
#[clap(
    author,
    version,
    about = "Issue a replacement token and retire the old one in a single step",
    long_about = None
)]
pub struct Config {
    /// Token id to replace, as shown by `admin token list`
    pub id: String,

    /// Write directly to the database named by -c, with no running server and no admin
    /// identity
    #[clap(long)]
    pub local: bool,
}

impl Config {
    pub async fn run<'a>(&'a self, cfg: &Cli) {
        if self.id == AccessTokenService::LEGACY_ID {
            eprintln!(
                "`legacy` cannot be rotated: it has no identifier to replace. Run \
                 `admin token mint`, update your mods, then `admin token revoke legacy`."
            );
            std::process::exit(1);
        }

        let rotated = if self.local {
            self.rotate_locally(cfg).await
        } else {
            self.rotate_remotely(cfg).await
        };

        eprintln!(
            "Token {} issued and {} retired. Every mod still holding the old value is \
             rejected until it is reconfigured.",
            rotated.id,
            rotated.revoked.as_deref().unwrap_or(&self.id)
        );
        println!("{}", rotated.token);

        if self.local {
            eprintln!("A running server applies both changes within 15 seconds.");
        }
    }

    async fn rotate_locally(&self, cfg: &Cli) -> MintedTokenResponse {
        let db = match cfg.config.create_database_connection().await {
            Ok(conn) => conn,
            Err(e) => {
                eprintln!("Failed to connect to database: {}", e);
                std::process::exit(1);
            }
        };

        match AccessTokenService::rotate_in(&db, &self.id).await {
            Ok(rotated) => rotated,
            Err(e) => {
                eprintln!("Failed to rotate the token: {}", e);
                std::process::exit(1);
            }
        }
    }

    async fn rotate_remotely(&self, cfg: &Cli) -> MintedTokenResponse {
        let client = match AdminApiClient::from_active_identity(cfg.identity.as_deref()) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        };

        match client.rotate_access_token(&self.id).await {
            Ok(rotated) => rotated,
            Err(e) => {
                eprintln!("Failed to rotate the token: {}", e);
                std::process::exit(1);
            }
        }
    }
}
