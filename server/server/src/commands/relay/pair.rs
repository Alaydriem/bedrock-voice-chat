use std::time::Duration;

use clap::Parser;

use bvc_server_lib::services::PairingService;

use crate::commands::Cli;
use crate::commands::admin::AdminApiClient;

#[derive(Debug, Parser, Clone)]
#[clap(author, version, about = "Mint a single-use pairing code for a voice bridge", long_about = None)]
pub struct Config {
    /// Name for the bridge that redeems this code, shown by `relay peers`
    #[clap(short, long, default_value = "svc-bridge")]
    pub label: String,

    /// Minutes the code stays redeemable
    #[clap(long, default_value = "15")]
    pub ttl_minutes: u64,

    /// Mint directly against the database named by -c, with no running server and no
    /// admin identity
    #[clap(long)]
    pub local: bool,
}

impl Config {
    pub async fn run<'a>(&'a self, cfg: &Cli) {
        let ttl = Duration::from_secs(self.ttl_minutes * 60);

        let code = if self.local {
            self.mint_locally(cfg, ttl).await
        } else {
            self.mint_remotely(cfg, ttl).await
        };

        println!("{}", code);
        println!();
        println!(
            "This code is shown once and expires in {} minutes.",
            self.ttl_minutes
        );
        println!("Run /bvc peer {} at the game server console.", code);
    }

    async fn mint_locally(&self, cfg: &Cli, ttl: Duration) -> String {
        let db = match cfg.config.create_database_connection().await {
            Ok(conn) => conn,
            Err(e) => {
                eprintln!("Failed to connect to database: {}", e);
                std::process::exit(1);
            }
        };

        match PairingService::mint(&db, &self.label, ttl).await {
            Ok(code) => code,
            Err(e) => {
                eprintln!("Failed to mint a pairing code: {}", e);

                // Migrations run in a Rocket fairing at server startup, not here. A
                // database the server has never opened has no tables, and the driver's
                // own message names the table rather than the cause.
                if e.to_string().contains("no such table") {
                    eprintln!();
                    eprintln!(
                        "This database has not been migrated. Start the server once, or run \
                         the migration binary against it, then try again."
                    );
                }

                std::process::exit(1);
            }
        }
    }

    async fn mint_remotely(&self, cfg: &Cli, ttl: Duration) -> String {
        let client = match AdminApiClient::from_active_identity(cfg.identity.as_deref()) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("{}", e);
                eprintln!();
                eprintln!(
                    "To mint without an identity, run this on the server host with --local \
                     and -c pointing at its config."
                );
                std::process::exit(1);
            }
        };

        match client.relay_pair(&self.label, ttl.as_secs()).await {
            Ok(resp) => resp.code,
            Err(e) => {
                eprintln!("Failed to mint a pairing code: {}", e);
                std::process::exit(1);
            }
        }
    }
}
