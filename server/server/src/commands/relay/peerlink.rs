use clap::Parser;

use bvc_server_lib::relay::PeerBlock;

use crate::commands::Cli;
use crate::commands::admin::AdminApiClient;
use crate::commands::admin::AdminApiError;

#[derive(Debug, Parser, Clone)]
#[clap(author, version, about = "Print this server's peer link", long_about = None)]
pub struct Config {
    /// Block label to render in the example, matching what the far side will call this server
    #[clap(short, long, default_value = "bvc-server")]
    pub label: String,
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

        match client.relay_peerlink().await {
            Ok(resp) => {
                println!("{}", resp.peerlink);
                println!();
                println!("node {}", resp.node_id);
                println!();
                println!("Add this inside the `server` block of the other server's config.hcl:");
                println!();
                print!("{}", PeerBlock::render(&self.label, &resp.peerlink));
            }
            Err(AdminApiError::NotFound) => {
                eprintln!(
                    "Peering is not configured on this server. Add a `peer` block to \
                     config.hcl and restart; the peer endpoint binds only when one exists."
                );
                std::process::exit(1);
            }
            Err(e) => {
                eprintln!("Failed to read the peer link: {}", e);
                std::process::exit(1);
            }
        }
    }
}
