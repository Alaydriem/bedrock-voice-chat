use std::sync::Arc;

use clap::Parser;
use common::Game;
use entity::player;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use super::super::Config as StateConfig;
use bvc_server_lib::services::{CertificateService, PlayerRegistrarService};

#[derive(Debug, Parser, Clone)]
#[clap(author, version, about = "Add a player to the server", long_about = None)]
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
        // Create database connection
        let db = match cfg.config.create_database_connection().await {
            Ok(conn) => conn,
            Err(e) => {
                eprintln!("Failed to connect to database: {}", e);
                return;
            }
        };

        // Check if player already exists
        let existing = player::Entity::find()
            .filter(player::Column::Gamertag.eq(self.player.clone()))
            .filter(player::Column::Game.eq(self.game.clone()))
            .one(&db)
            .await;

        match existing {
            Ok(Some(_)) => {
                println!(
                    "Player '{}' already exists for game '{}'",
                    self.player, self.game
                );
                return;
            }
            Ok(None) => {
                // Player doesn't exist, proceed with creation
            }
            Err(e) => {
                eprintln!("Failed to query database: {}", e);
                return;
            }
        }

        // Create certificate service
        let cert_service = match CertificateService::new_shared(&cfg.config.server.tls.certs_path) {
            Ok(cs) => cs,
            Err(e) => {
                eprintln!("Failed to initialize certificate service: {}", e);
                return;
            }
        };

        // Create player registrar service and add the player
        let db_arc = Arc::new(db);
        let registrar = PlayerRegistrarService::new(db_arc, cert_service);

        registrar.create_player(&self.player, &self.game).await;
        println!(
            "Successfully added player '{}' for game '{}'",
            self.player, self.game
        );
    }
}
