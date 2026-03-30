use clap::Parser;
use common::Game;
use entity::player;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use super::super::Cli;
use bvc_server_lib::services::AuthCodeService;

#[derive(Debug, Parser, Clone)]
#[clap(author, version, about = "Generate a login code for a player", long_about = None)]
pub struct Config {
    /// The player's gamertag
    #[clap(short, long)]
    pub player: String,

    /// The game type (minecraft or hytale)
    #[clap(short, long, value_enum)]
    pub game: Game,

    /// How long the code is valid for, in seconds
    #[clap(short, long, default_value = "3600")]
    pub duration: u64,
}

impl Config {
    pub async fn run<'a>(&'a self, cfg: &Cli) {
        let db = match cfg.config.create_database_connection().await {
            Ok(conn) => conn,
            Err(e) => {
                eprintln!("Failed to connect to database: {}", e);
                return;
            }
        };

        // Verify the player exists
        let existing = player::Entity::find()
            .filter(player::Column::Gamertag.eq(self.player.clone()))
            .filter(player::Column::Game.eq(self.game.clone()))
            .one(&db)
            .await;

        let player_record = match existing {
            Ok(Some(p)) => p,
            Ok(None) => {
                eprintln!(
                    "Player '{}' not found for game '{}'. Add the player first with `bvc user add`.",
                    self.player, self.game
                );
                return;
            }
            Err(e) => {
                eprintln!("Failed to query database: {}", e);
                return;
            }
        };

        match AuthCodeService::generate_code(&db, player_record.id, self.duration).await {
            Ok(code) => {
                println!("Code: {}", code);
                println!("Player: {} ({})", self.player, self.game);
                println!("Expires in: {}s", self.duration);
            }
            Err(e) => {
                eprintln!("Failed to generate code: {}", e);
            }
        }
    }
}
