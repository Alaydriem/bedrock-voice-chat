use clap::Parser;
use common::response::admin::LegacyTokenResponse;

use bvc_server_lib::runtime::{SecretName, SecretStore};

use crate::commands::Cli;
use crate::commands::admin::AdminApiClient;

const DEPRECATION_WARNING: &str = "\
WARNING: this token is deprecated.

It is stored in plaintext and every mod shares it, so it cannot be retired for one
server without breaking the rest. Run `bvc-server admin token mint` to issue a
replacement, update your Addon or mod configuration, then run
`bvc-server admin token revoke legacy`.
";

#[derive(Debug, Parser, Clone)]
#[clap(
    author,
    version,
    about = "Print the deprecated pre-identifier access token",
    long_about = None
)]
pub struct Config {
    /// Read directly from the database named by -c, with no running server and no admin
    /// identity
    #[clap(long)]
    pub local: bool,
}

impl Config {
    pub async fn run<'a>(&'a self, cfg: &Cli) {
        let legacy = if self.local {
            self.read_locally(cfg).await
        } else {
            self.read_remotely(cfg).await
        };

        let Some(token) = legacy.token.filter(|value| !value.is_empty()) else {
            eprintln!(
                "This server has no legacy access token. Run `admin token mint` to issue one."
            );
            std::process::exit(1);
        };

        eprintln!("{}", DEPRECATION_WARNING);
        if legacy.configured {
            eprintln!(
                "This value comes from the environment or config.hcl, so it is re-applied at \
                 every startup. Remove it there once your mods use a minted token.\n"
            );
        }

        println!("{token}");
    }

    async fn read_locally(&self, cfg: &Cli) -> LegacyTokenResponse {
        let configured = cfg.config.server.minecraft.access_token.trim();
        if !configured.is_empty() {
            return LegacyTokenResponse {
                token: Some(configured.to_string()),
                configured: true,
            };
        }

        let db = match cfg.config.create_database_connection().await {
            Ok(conn) => conn,
            Err(e) => {
                eprintln!("Failed to connect to database: {}", e);
                std::process::exit(1);
            }
        };

        match SecretStore::read(&db, SecretName::MinecraftAccessToken).await {
            Ok(token) => LegacyTokenResponse {
                token,
                configured: false,
            },
            Err(e) => {
                eprintln!("Failed to read the legacy token: {}", e);
                std::process::exit(1);
            }
        }
    }

    async fn read_remotely(&self, cfg: &Cli) -> LegacyTokenResponse {
        let client = match AdminApiClient::from_active_identity(cfg.identity.as_deref()) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        };

        match client.legacy_access_token().await {
            Ok(legacy) => legacy,
            Err(e) => {
                eprintln!("Failed to read the legacy token: {}", e);
                std::process::exit(1);
            }
        }
    }
}
