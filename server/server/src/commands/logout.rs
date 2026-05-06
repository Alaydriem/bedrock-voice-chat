use clap::Parser;

use crate::identity::{IdentityResolver, IdentityStore};

#[derive(Debug, Parser, Clone)]
#[clap(author, version, about = "Remove the active CLI identity (cert, key, CA + metadata)", long_about = None)]
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

        match IdentityStore::delete(&slot) {
            Ok(_) => println!("Logged out {} ({})", slot.gamertag, slot.game.as_str()),
            Err(e) => {
                eprintln!("Failed to delete identity: {}", e);
                std::process::exit(1);
            }
        }
    }
}
