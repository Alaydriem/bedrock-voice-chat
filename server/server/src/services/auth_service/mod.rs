//! Authentication service for building login responses

mod auth_error;

use std::path::Path;

use common::{
    response::LoginResponse,
    structs::{
        config::Keypair,
        permission::ServerPermissions,
    },
    Game,
};
use entity::player;
use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

pub use auth_error::AuthError;

use crate::config::Server;
use crate::services::permission_service::PermissionService;

/// Service for authentication operations
pub struct AuthService;

impl AuthService {
    /// Resolve a player from an mTLS certificate CN.
    /// Supports both new format "game:gamertag" and legacy "gamertag" (no game prefix).
    pub async fn player_from_certificate<C: ConnectionTrait>(
        cert: &rocket::mtls::Certificate<'_>,
        conn: &C,
        game_hint: Option<&str>,
    ) -> Result<player::Model, rocket::http::Status> {
        let cn = match cert.subject().common_name() {
            Some(cn) => cn,
            None => {
                return Err(rocket::http::Status::Forbidden);
            }
        };

        let (game_filter, gamertag) = match cn.split_once(':') {
            Some((game, name)) => {
                let effective_game = game_hint
                    .map(|g| g.to_lowercase())
                    .unwrap_or_else(|| game.to_lowercase());
                (Some(effective_game), name.to_string())
            }
            None => {
                let effective_game = game_hint
                    .map(|g| g.to_lowercase())
                    .unwrap_or_else(|| "minecraft".to_string());
                tracing::info!("player_from_certificate: legacy cert gamertag={}, effective_game={}", cn, effective_game);
                (Some(effective_game), cn.to_string())
            }
        };

        let mut query = player::Entity::find()
            .filter(player::Column::Gamertag.eq(&gamertag));
        if let Some(ref game) = game_filter {
            query = query.filter(player::Column::Game.eq(game));
        }

        match query.one(conn).await {
            Ok(Some(player)) => Ok(player),
            Ok(None) => {
                tracing::error!(
                    "player_from_certificate: no player found for gamertag={:?}, game_filter={:?}",
                    gamertag,
                    game_filter
                );
                Err(rocket::http::Status::Forbidden)
            }
            Err(e) => {
                tracing::error!("player_from_certificate: DB error: {}", e);
                Err(rocket::http::Status::InternalServerError)
            }
        }
    }

    /// Build a LoginResponse for an authenticated player
    pub async fn build_login_response<C: ConnectionTrait>(
        conn: &C,
        config: &Server,
        permission_service: Option<&PermissionService>,
        gamertag: String,
        gamerpic: String,
        game: Game,
    ) -> Result<LoginResponse, AuthError> {
        let player_record = player::Entity::find()
            .filter(player::Column::Gamertag.eq(gamertag.clone()))
            .filter(player::Column::Game.eq(game.clone()))
            .one(conn)
            .await
            .map_err(|e| AuthError::DatabaseError(e.to_string()))?;

        let actual = match player_record {
            Some(p) => p,
            None => {
                tracing::info!("Player {} ({:?}) not found in database", gamertag, game);
                return Err(AuthError::PlayerNotFound);
            }
        };

        if actual.gamerpic.as_ref() != Some(&gamerpic) {
            let mut player_active: player::ActiveModel = actual.clone().into();
            player_active.gamerpic = ActiveValue::Set(Some(gamerpic.clone()));
            player_active.update(conn).await.map_err(|e| {
                tracing::error!("Failed to update gamerpic: {}", e);
                AuthError::DatabaseError(e.to_string())
            })?;
            tracing::debug!("Updated gamerpic for player {}", gamertag);
        }

        if actual.banished {
            tracing::info!("Player {} is banished", gamertag);
            return Err(AuthError::PlayerBanished);
        }

        let kp = actual.get_keypair().map_err(|e| {
            tracing::error!("Failed to get keypair: {}", e);
            AuthError::CertificateError(e.to_string())
        })?;

        let sp = actual.get_signature().map_err(|e| {
            tracing::error!("Failed to get signature: {}", e);
            AuthError::CertificateError(e.to_string())
        })?;

        let certificate_ca =
            std::fs::read_to_string(Path::new(&format!("{}/ca.crt", config.tls.certs_path)))
                .map_err(|e| {
                    tracing::error!("Failed to read CA certificate: {}", e);
                    AuthError::CertificateError(e.to_string())
                })?;

        let decoded_gamerpic = crate::services::GamerpicDecoder::decode(Some(gamerpic))
            .unwrap_or_default();

        let server_permissions = if let Some(perm_service) = permission_service {
            let allowed = perm_service.evaluate_all(conn, actual.id).await;
            Some(ServerPermissions { allowed })
        } else {
            None
        };

        Ok(LoginResponse::new(
            gamertag,
            decoded_gamerpic,
            Keypair {
                pk: kp.get_public_key(),
                sk: kp.get_public_key(),
            },
            Keypair {
                pk: sp.get_public_key(),
                sk: sp.get_public_key(),
            },
            actual.certificate,
            actual.certificate_key,
            certificate_ca,
            config.quic_port.to_string(),
            server_permissions,
        ))
    }
}
