use clap::Parser;
use common::Game;
use entity::{player, player_permission};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::commands::Config as StateConfig;

#[derive(Debug, Parser, Clone)]
#[clap(author, version, about = "List permission overrides for a player", long_about = None)]
pub struct Config {
    /// The player's gamertag
    #[clap(short, long)]
    pub player: String,

    /// The game type (minecraft or hytale)
    #[clap(short, long, value_enum)]
    pub game: Game,
}

impl Config {
    pub async fn run<'a>(&'a self, cfg: &StateConfig) {
        let db = match cfg.config.create_database_connection().await {
            Ok(conn) => conn,
            Err(e) => {
                eprintln!("Failed to connect to database: {}", e);
                return;
            }
        };

        let player_model = match player::Entity::find()
            .filter(player::Column::Gamertag.eq(self.player.clone()))
            .filter(player::Column::Game.eq(self.game.clone()))
            .one(&db)
            .await
        {
            Ok(Some(p)) => p,
            Ok(None) => {
                eprintln!(
                    "Player '{}' not found for game '{}'",
                    self.player, self.game
                );
                return;
            }
            Err(e) => {
                eprintln!("Failed to query database: {}", e);
                return;
            }
        };

        let permissions = player_permission::Entity::find()
            .filter(player_permission::Column::PlayerId.eq(player_model.id))
            .all(&db)
            .await;

        match permissions {
            Ok(records) => {
                if records.is_empty() {
                    println!(
                        "No permission overrides for player '{}' (using config defaults)",
                        self.player
                    );
                } else {
                    println!("Permission overrides for player '{}':", self.player);
                    for record in records {
                        let effect_str = if record.effect & 1 == 1 {
                            "allow"
                        } else {
                            "deny"
                        };
                        println!("  {} = {}", record.permission, effect_str);
                    }
                }
            }
            Err(e) => eprintln!("Failed to query permissions: {}", e),
        }
    }
}
