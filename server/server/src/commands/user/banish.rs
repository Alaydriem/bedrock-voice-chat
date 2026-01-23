use clap::Parser;
use common::Game;
use entity::player;
use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter};

use crate::commands::Config as StateConfig;

#[derive(Debug, Parser, Clone)]
#[clap(author, version, about = "Banish or unbanish a player", long_about = None)]
pub struct Config {
    /// The player's gamertag
    #[clap(short, long)]
    pub player: String,

    /// The game type (minecraft or hytale)
    #[clap(short, long, value_enum)]
    pub game: Game,

    /// Set to true to banish, false to unbanish
    #[clap(short, long, default_value = "true")]
    pub banish: bool,
}

impl Config {
    pub async fn run<'a>(&'a self, cfg: &StateConfig) {
        // Create database connection
        let db = match cfg.config.create_database_connection().await {
            Ok(conn) => conn,
            Err(e) => {
                eprintln!("Failed to connect to database: {}", e);
                return;
            }
        };

        // Find the player
        let player_result = player::Entity::find()
            .filter(player::Column::Gamertag.eq(self.player.clone()))
            .filter(player::Column::Game.eq(self.game.clone()))
            .one(&db)
            .await;

        let player_model = match player_result {
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

        // Update the banished status
        let mut active_model: player::ActiveModel = player_model.into();
        active_model.banished = ActiveValue::Set(self.banish);

        match active_model.update(&db).await {
            Ok(_) => {
                let action = if self.banish { "banished" } else { "unbanished" };
                println!(
                    "Successfully {} player '{}' for game '{}'",
                    action, self.player, self.game
                );
            }
            Err(e) => {
                eprintln!("Failed to update player: {}", e);
            }
        }
    }
}
