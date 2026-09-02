use std::sync::Arc;

use clap::Parser;
use common::Game;
use common::structs::permission::{Permission, PermissionEffect};
use entity::player;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::commands::Cli;
use bvc_server_lib::services::{CertificateService, PermissionService, PlayerRegistrarService};

#[derive(Debug, Parser, Clone)]
#[clap(author, version, about = "Grant the `admin` permission to a player. Runs locally against the DB; the only non-server CLI command that does.", long_about = None)]
pub struct Config {
    /// Player gamertag
    #[clap(short = 'p', long)]
    pub player: String,

    /// Game (minecraft)
    #[clap(short, long, value_enum, default_value_t = Game::Minecraft)]
    pub game: Game,
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
                eprintln!("Failed to query database: {}", e);
                std::process::exit(1);
            }
        };

        let player_record = match existing {
            Some(p) => p,
            // Created through the registrar rather than by inserting a row directly: a
            // player needs a signed certificate, an ncryptf keypair and a signature keypair
            // to authenticate at all, and only the registrar mints them.
            None => self.create_player(cfg, &db).await,
        };

        match PermissionService::set_override(
            &db,
            player_record.id,
            Permission::Admin.as_str(),
            PermissionEffect::Allow,
        )
        .await
        {
            Ok(()) => println!(
                "Granted admin permission to {} ({}).",
                self.player, self.game
            ),
            Err(e) => {
                eprintln!("Failed to grant admin permission: {}", e);
                std::process::exit(1);
            }
        }
    }

    async fn create_player(&self, cfg: &Cli, db: &sea_orm::DatabaseConnection) -> player::Model {
        let cert_service = match CertificateService::new_shared(&cfg.config.server.tls.certs_path) {
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
}
