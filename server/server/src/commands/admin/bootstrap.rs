use clap::Parser;
use common::Game;
use common::structs::permission::{Permission, PermissionEffect};
use entity::player;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::commands::Cli;
use bvc_server_lib::services::PermissionService;

#[derive(Debug, Parser, Clone)]
#[clap(author, version, about = "Grant the `admin` permission to a player. Runs locally against the DB; the only non-server CLI command that does.", long_about = None)]
pub struct Config {
    /// Player gamertag
    #[clap(short = 'p', long)]
    pub player: String,

    /// Game (minecraft)
    #[clap(short, long, value_enum)]
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

        let player_record = match player::Entity::find()
            .filter(player::Column::Gamertag.eq(self.player.clone()))
            .filter(player::Column::Game.eq(self.game.clone()))
            .one(&db)
            .await
        {
            Ok(Some(p)) => p,
            Ok(None) => {
                eprintln!(
                    "Player '{}' not found for game '{}'. Add the player first (e.g. via the desktop client).",
                    self.player, self.game
                );
                std::process::exit(1);
            }
            Err(e) => {
                eprintln!("Failed to query database: {}", e);
                std::process::exit(1);
            }
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
}
