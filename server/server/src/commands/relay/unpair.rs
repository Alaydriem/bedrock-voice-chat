use clap::Parser;

use bvc_server_lib::services::PairingService;

use crate::commands::Cli;
use crate::commands::admin::AdminApiClient;

#[derive(Debug, Parser, Clone)]
#[clap(author, version, about = "Revoke a paired voice bridge", long_about = None)]
pub struct Config {
    /// The name the code was minted under, as shown by `relay peers`
    pub label: String,

    /// Revoke directly in the database named by -c, with no running server and no admin
    /// identity
    #[clap(long)]
    pub local: bool,
}

impl Config {
    pub async fn run<'a>(&'a self, cfg: &Cli) {
        let removed = if self.local {
            self.revoke_locally(cfg).await
        } else {
            self.revoke_remotely(cfg).await
        };

        if removed == 0 {
            println!("No paired bridge is named {:?}.", self.label);
            return;
        }

        println!("Revoked {} grant(s) for {:?}.", removed, self.label);

        if self.local {
            // The running table is loaded at startup and updated in memory by the
            // redemptions it saw. A row deleted underneath it is not one of those.
            eprintln!(
                "Note: a running server keeps this peer authorized until it restarts. Run \
                 without --local to revoke it immediately."
            );
        }
    }

    async fn revoke_locally(&self, cfg: &Cli) -> u64 {
        let db = match cfg.config.create_database_connection().await {
            Ok(conn) => conn,
            Err(e) => {
                eprintln!("Failed to connect to database: {}", e);
                std::process::exit(1);
            }
        };

        match PairingService::revoke(&db, &self.label).await {
            Ok(removed) => removed,
            Err(e) => {
                eprintln!("Failed to revoke the paired peer: {}", e);
                std::process::exit(1);
            }
        }
    }

    async fn revoke_remotely(&self, cfg: &Cli) -> u64 {
        let client = match AdminApiClient::from_active_identity(cfg.identity.as_deref()) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        };

        // The route answers with what remains, so the count is derived rather than
        // reported: asking for the list first would race a concurrent pairing.
        let before = match client.relay_paired().await {
            Ok(resp) => resp.peers.len(),
            Err(e) => {
                eprintln!("Failed to list paired peers: {}", e);
                std::process::exit(1);
            }
        };

        match client.relay_unpair(&self.label).await {
            Ok(resp) => (before - resp.peers.len()) as u64,
            Err(e) => {
                eprintln!("Failed to revoke the paired peer: {}", e);
                std::process::exit(1);
            }
        }
    }
}
