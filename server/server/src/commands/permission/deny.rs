use clap::Parser;
use common::Game;
use entity::{player, player_permission};
use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter};

use crate::commands::Config as StateConfig;

#[derive(Debug, Parser, Clone)]
#[clap(author, version, about = "Deny a permission for a player", long_about = None)]
pub struct Config {
    /// The player's gamertag
    #[clap(short, long)]
    pub player: String,

    /// The game type (minecraft or hytale)
    #[clap(short, long, value_enum)]
    pub game: Game,

    /// The permission to deny (e.g. audio_upload, audio_delete)
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

        // Check if override already exists
        let existing = player_permission::Entity::find()
            .filter(player_permission::Column::PlayerId.eq(player_model.id))
            .filter(player_permission::Column::Permission.eq(self.permission.clone()))
            .one(&db)
            .await;

        match existing {
            Ok(Some(record)) => {
                // Update existing record
                let mut active: player_permission::ActiveModel = record.into();
                active.effect = ActiveValue::Set(0);
                match active.update(&db).await {
                    Ok(_) => println!(
                        "Denied permission '{}' for player '{}'",
                        self.permission, self.player
                    ),
                    Err(e) => eprintln!("Failed to update permission: {}", e),
                }
            }
            Ok(None) => {
                // Insert new record
                let now = common::ncryptflib::rocket::Utc::now().timestamp();
                let active = player_permission::ActiveModel {
                    id: ActiveValue::NotSet,
                    player_id: ActiveValue::Set(player_model.id),
                    permission: ActiveValue::Set(self.permission.clone()),
                    effect: ActiveValue::Set(0),
                    created_at: ActiveValue::Set(now),
                };
                match active.insert(&db).await {
                    Ok(_) => println!(
                        "Denied permission '{}' for player '{}'",
                        self.permission, self.player
                    ),
                    Err(e) => eprintln!("Failed to insert permission: {}", e),
                }
            }
            Err(e) => eprintln!("Failed to query permissions: {}", e),
        }
    }
}
