use std::sync::Arc;

use clap::Parser;
use common::Game;
use entity::player;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::commands::Cli;
use bvc_server_lib::services::{AuthCodeService, CertificateService, PlayerRegistrarService};

#[derive(Debug, Parser, Clone)]
#[clap(author, version, about = "Generate a one-time login code locally against the DB. Creates the player record if missing. Server-host only.", long_about = None)]
pub struct Config {
    /// Player gamertag
    #[clap(short = 'p', long)]
    pub player: String,

    /// Game (minecraft or hytale)
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
                std::process::exit(1);
            }
        };

        let existing = match player::Entity::find()
            .filter(player::Column::Gamertag.eq(self.player.clone()))
            .filter(player::Column::Game.eq(self.game.clone()))
            .one(&db)
            .await
        {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Failed to query player: {}", e);
                std::process::exit(1);
            }
        };

        let player_record = match existing {
            Some(p) => p,
            None => {
                let cert_service = match CertificateService::new_shared(
                    &cfg.config.server.tls.certs_path,
                ) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Failed to initialize certificate service: {}", e);
                        std::process::exit(1);
                    }
                };

                let registrar = PlayerRegistrarService::new(Arc::new(db.clone()), cert_service);
                match registrar
                    .create_player(&self.player, &self.game, None)
                    .await
                {
                    Ok(p) => {
                        println!(
                            "Created player record for {} ({}).",
                            self.player,
                            self.game.as_str()
                        );
                        p
                    }
                    Err(e) => {
                        eprintln!("Failed to create player record: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        };

        match AuthCodeService::generate_code(&db, player_record.id, self.duration).await {
            Ok(code) => {
                println!("Code: {}", code);
                println!("Player: {} ({})", self.player, self.game.as_str());
                println!("Expires in: {}s", self.duration);
            }
            Err(e) => {
                eprintln!("Failed to generate code: {}", e);
                std::process::exit(1);
            }
        }
    }
}
