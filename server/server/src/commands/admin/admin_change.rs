use std::sync::Arc;

use common::Game;
use common::request::admin::AdminPermissionRequest;
use common::structs::permission::AdminAction;

use crate::commands::Cli;
use bvc_server_lib::services::{
    AdminPermissionOutcome, AdminPermissionService, CertificateService, PlayerRegistrarService,
};

/// Changes a player's `admin` permission directly against the database, for an operator at
/// the server host's console.
pub struct AdminChange;

impl AdminChange {
    pub async fn run(cfg: &Cli, player: &str, game: &Game, action: AdminAction) {
        let db = match cfg.config.create_database_connection().await {
            Ok(conn) => Arc::new(conn),
            Err(e) => Self::fail(player, &format!("connecting to the database: {e}")),
        };
        let cert_service = match CertificateService::new_shared(&cfg.config.server.tls.certs_path)
        {
            Ok(service) => service,
            Err(e) => Self::fail(player, &format!("loading the certificate authority: {e}")),
        };
        let service = AdminPermissionService::new(
            db.clone(),
            PlayerRegistrarService::new(db, cert_service),
        );
        let request = AdminPermissionRequest {
            gamertag: player.to_string(),
            game: game.clone(),
            action,
        };

        let game = game.as_str();
        match service.apply(&request).await {
            Ok(AdminPermissionOutcome::Applied) => match action {
                AdminAction::Revoke => println!("Revoked admin from {player} ({game})."),
                _ => println!("Granted admin to {player} ({game})."),
            },
            Ok(AdminPermissionOutcome::NothingToRevoke) => {
                println!("{player} ({game}) has no admin grant to revoke.")
            }
            Ok(AdminPermissionOutcome::PlayerNotFound) => {
                println!("{player} ({game}) is not registered.");
                std::process::exit(1);
            }
            Err(e) => Self::fail(player, &e.to_string()),
        }
    }

    fn fail(player: &str, reason: &str) -> ! {
        eprintln!("Failed to change admin for {player}: {reason}");
        std::process::exit(1);
    }
}
