use clap::Parser;
use common::Game;
use entity::{player, player_permission};
use sea_orm::{ColumnTrait, EntityTrait, ModelTrait, QueryFilter};

use crate::commands::Config as StateConfig;

#[derive(Debug, Parser, Clone)]
#[clap(author, version, about = "Clear a permission override (fall back to config default)", long_about = None)]
pub struct Config {
    /// The player's gamertag
    #[clap(short, long)]
    pub player: String,

    /// The game type (minecraft or hytale)
    #[clap(short, long, value_enum)]
    pub game: Game,

    /// The permission to clear (e.g. audio_upload, audio_delete)
    #[clap(long)]
    pub permission: String,
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

        let existing = player_permission::Entity::find()
            .filter(player_permission::Column::PlayerId.eq(player_model.id))
            .filter(player_permission::Column::Permission.eq(self.permission.clone()))
            .one(&db)
            .await;

        match existing {
            Ok(Some(record)) => match record.delete(&db).await {
                Ok(_) => println!(
                    "Cleared permission override '{}' for player '{}' (will use config default)",
                    self.permission, self.player
                ),
                Err(e) => eprintln!("Failed to delete permission override: {}", e),
            },
            Ok(None) => {
                println!(
                    "No override found for permission '{}' on player '{}'",
                    self.permission, self.player
                );
            }
            Err(e) => eprintln!("Failed to query permissions: {}", e),
        }
    }
}
