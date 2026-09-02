use clap::Parser;

use bvc_server_lib::runtime::{SecretName, SecretStore};
use bvc_server_lib::services::AccessTokenService;

use crate::commands::Cli;
use crate::commands::admin::AdminApiClient;

#[derive(Debug, Parser, Clone)]
#[clap(author, version, about = "Retire a game server access token", long_about = None)]
pub struct Config {
    /// Token id, as shown by `admin token list`. `legacy` removes the pre-identifier scalar
    pub id: String,

    /// Write directly to the database named by -c, with no running server and no admin
    /// identity
    #[clap(long)]
    pub local: bool,
}

impl Config {
    pub async fn run<'a>(&'a self, cfg: &Cli) {
        let revoked = if self.local {
            self.revoke_locally(cfg).await
        } else {
            self.revoke_remotely(cfg).await
        };

        if !revoked {
            eprintln!("No token with id `{}`.", self.id);
            std::process::exit(1);
        }

        println!("Revoked {}.", self.id);
        if self.local {
            eprintln!("A running server stops accepting it within 15 seconds.");
        }
    }

    async fn revoke_locally(&self, cfg: &Cli) -> bool {
        let db = match cfg.config.create_database_connection().await {
            Ok(conn) => conn,
            Err(e) => {
                eprintln!("Failed to connect to database: {}", e);
                std::process::exit(1);
            }
        };

        if self.id == AccessTokenService::LEGACY_ID {
            if !cfg.config.server.minecraft.access_token.trim().is_empty() {
                eprintln!(
                    "The legacy token comes from the environment or config.hcl. Change it \
                     there: startup writes that value back."
                );
                std::process::exit(1);
            }

            return match SecretStore::delete(&db, SecretName::MinecraftAccessToken).await {
                Ok(removed) => removed,
                Err(e) => {
                    eprintln!("Failed to remove the legacy token: {}", e);
                    std::process::exit(1);
                }
            };
        }

        match AccessTokenService::revoke_in(&db, &self.id).await {
            Ok(revoked) => revoked,
            Err(e) => {
                eprintln!("Failed to revoke the token: {}", e);
                std::process::exit(1);
            }
        }
    }

    async fn revoke_remotely(&self, cfg: &Cli) -> bool {
        let client = match AdminApiClient::from_active_identity(cfg.identity.as_deref()) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        };

        match client.revoke_access_token(&self.id).await {
            Ok(()) => true,
            Err(e) => {
                eprintln!("Failed to revoke the token: {}", e);
                std::process::exit(1);
            }
        }
    }
}
