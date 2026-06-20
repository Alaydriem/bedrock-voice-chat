use clap::Parser;

use crate::commands::admin_api_client::AdminApiClient;
use crate::identity::{IdentityResolver, IdentityStore};

#[derive(Debug, Parser, Clone)]
#[clap(author, version, about = "Print the active identity (calls /api/auth/introspect)", long_about = None)]
pub struct Config {}

impl Config {
    pub async fn run(&self, identity_flag: Option<&str>) {
        let slot = match IdentityResolver::active(identity_flag) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        };

        let identity = match IdentityStore::load(&slot) {
            Ok(i) => i,
            Err(e) => {
                eprintln!("Failed to load identity: {}", e);
                std::process::exit(1);
            }
        };

        let client = match AdminApiClient::new(&identity) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to build HTTP client: {}", e);
                std::process::exit(1);
            }
        };

        match client.introspect().await {
            Ok(resp) => {
                println!("Gamertag:     {}", resp.gamertag);
                println!("Game:         {}", resp.game.as_str());
                println!("Server:       {}", identity.server_url);
                if let Some(ts) = resp.cert_not_after {
                    println!("Cert expires: {} (epoch)", ts);
                }
                if resp.permissions.is_empty() {
                    println!("Permissions:  (none)");
                } else {
                    let perms: Vec<&str> = resp.permissions.iter().map(|p| p.as_str()).collect();
                    println!("Permissions:  {}", perms.join(", "));
                }
            }
            Err(e) => {
                eprintln!("Introspect failed: {}", e);
                eprintln!("Local identity slot: {}", slot.key());
                std::process::exit(1);
            }
        }
    }
}
