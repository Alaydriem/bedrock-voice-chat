use clap::Parser;

use bvc_server_lib::services::PairingService;
use common::response::admin::PairedPeer;

use crate::commands::Cli;
use crate::commands::admin::AdminApiClient;

#[derive(Debug, Parser, Clone)]
#[clap(author, version, about = "List the voice bridges that have paired with this server", long_about = None)]
pub struct Config {
    /// Read directly from the database named by -c, with no running server and no admin
    /// identity
    #[clap(long)]
    pub local: bool,
}

impl Config {
    pub async fn run<'a>(&'a self, cfg: &Cli) {
        let peers = if self.local {
            self.read_locally(cfg).await
        } else {
            self.read_remotely(cfg).await
        };

        if peers.is_empty() {
            println!(
                "No bridge has paired with this server. Mint a code with `relay pair`, then \
                 run /bvc peer <code> at the game server console."
            );
            return;
        }

        println!("{:<40}{}", "peer", "node");
        for peer in peers {
            println!("{:<40}{}", peer.label, peer.node_id);
        }
    }

    async fn read_locally(&self, cfg: &Cli) -> Vec<PairedPeer> {
        let db = match cfg.config.create_database_connection().await {
            Ok(conn) => conn,
            Err(e) => {
                eprintln!("Failed to connect to database: {}", e);
                std::process::exit(1);
            }
        };

        match PairingService::paired(&db).await {
            Ok(rows) => rows
                .into_iter()
                .map(|row| PairedPeer {
                    node_id: row.node_id,
                    label: row.label,
                    paired_at: row.paired_at,
                })
                .collect(),
            Err(e) => {
                eprintln!("Failed to list paired peers: {}", e);
                std::process::exit(1);
            }
        }
    }

    async fn read_remotely(&self, cfg: &Cli) -> Vec<PairedPeer> {
        let client = match AdminApiClient::from_active_identity(cfg.identity.as_deref()) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        };

        match client.relay_paired().await {
            Ok(resp) => resp.peers,
            Err(e) => {
                eprintln!("Failed to list paired peers: {}", e);
                std::process::exit(1);
            }
        }
    }
}
