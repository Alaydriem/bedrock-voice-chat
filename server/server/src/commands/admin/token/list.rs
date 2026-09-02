use clap::Parser;
use common::response::admin::AccessTokenRow;

use bvc_server_lib::runtime::{SecretName, SecretStore};
use bvc_server_lib::services::AccessTokenService;

use crate::commands::Cli;
use crate::commands::admin::AdminApiClient;

#[derive(Debug, Parser, Clone)]
#[clap(author, version, about = "List issued game server access tokens", long_about = None)]
pub struct Config {
    /// Read directly from the database named by -c, with no running server and no admin
    /// identity
    #[clap(long)]
    pub local: bool,
}

impl Config {
    pub async fn run<'a>(&'a self, cfg: &Cli) {
        let (tokens, legacy_configured) = if self.local {
            self.read_locally(cfg).await
        } else {
            self.read_remotely(cfg).await
        };

        if tokens.is_empty() {
            println!(
                "No access token has been issued. Run `admin token mint` and put the printed \
                 value in your Addon or mod configuration."
            );
            return;
        }

        println!("{:<12}{:<14}{}", "id", "created", "state");
        for token in tokens {
            let state = match (token.id.as_str(), token.revoked_at) {
                (_, Some(at)) => format!("revoked {at}"),
                (AccessTokenService::LEGACY_ID, None) if legacy_configured => {
                    "active (configured)".to_string()
                }
                (AccessTokenService::LEGACY_ID, None) => "active (generated)".to_string(),
                (_, None) => "active".to_string(),
            };
            println!("{:<12}{:<14}{}", token.id, token.created_at, state);
        }
    }

    async fn read_locally(&self, cfg: &Cli) -> (Vec<AccessTokenRow>, bool) {
        let db = match cfg.config.create_database_connection().await {
            Ok(conn) => conn,
            Err(e) => {
                eprintln!("Failed to connect to database: {}", e);
                std::process::exit(1);
            }
        };

        let mut tokens = match AccessTokenService::list_in(&db).await {
            Ok(rows) => rows,
            Err(e) => {
                eprintln!("Failed to list tokens: {}", e);
                std::process::exit(1);
            }
        };

        let configured = !cfg.config.server.minecraft.access_token.trim().is_empty();
        let stored = SecretStore::read(&db, SecretName::MinecraftAccessToken)
            .await
            .unwrap_or(None);

        if configured || stored.is_some() {
            tokens.insert(
                0,
                AccessTokenRow {
                    id: AccessTokenService::LEGACY_ID.to_string(),
                    created_at: 0,
                    revoked_at: None,
                },
            );
        }

        (tokens, configured)
    }

    async fn read_remotely(&self, cfg: &Cli) -> (Vec<AccessTokenRow>, bool) {
        let client = match AdminApiClient::from_active_identity(cfg.identity.as_deref()) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        };

        let tokens = match client.list_access_tokens().await {
            Ok(resp) => resp.tokens,
            Err(e) => {
                eprintln!("Failed to list tokens: {}", e);
                std::process::exit(1);
            }
        };

        let configured = client
            .legacy_access_token()
            .await
            .map(|legacy| legacy.configured)
            .unwrap_or(false);

        (tokens, configured)
    }
}
