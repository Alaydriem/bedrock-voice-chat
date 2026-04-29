use clap::Parser;
use common::Game;

use crate::commands::identity::{IdentityResolver, IdentitySlot, IdentityStore};

#[derive(Debug, Parser, Clone)]
#[clap(author, version, about = "Remove a stored CLI identity (cert, key, CA + metadata)", long_about = None)]
pub struct Config {
    /// Gamertag to log out (defaults to active identity)
    #[clap(short = 'p', long)]
    pub gamertag: Option<String>,

    /// Game (minecraft or hytale)
    #[clap(short, long, value_enum)]
    pub game: Option<Game>,
}

impl Config {
    pub async fn run(&self, identity_flag: Option<&str>) {
        let slot = match (self.gamertag.clone(), self.game.clone()) {
            (Some(gamertag), Some(game)) => IdentitySlot::new(gamertag, game),
            (None, None) => match IdentityResolver::active(identity_flag) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            },
            _ => {
                eprintln!("--gamertag and --game must be provided together");
                std::process::exit(1);
            }
        };

        match IdentityStore::delete(&slot) {
            Ok(_) => println!(
                "Logged out {} ({})",
                slot.gamertag,
                slot.game.as_str()
            ),
            Err(e) => {
                eprintln!("Failed to delete identity: {}", e);
                std::process::exit(1);
            }
        }
    }
}
