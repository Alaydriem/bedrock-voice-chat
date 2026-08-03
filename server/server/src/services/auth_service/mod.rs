//! Authentication service for building login responses

mod auth_error;
mod code_login_error;

use std::path::Path;
use std::sync::Arc;

use common::{
    Game,
    request::CodeLoginRequest,
    response::LoginResponse,
    structs::{config::Keypair, permission::ServerPermissions},
};
use entity::player;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
};

pub use auth_error::AuthError;
pub use code_login_error::CodeLoginError;

use crate::config::Server;
use crate::services::auth_code_service::AuthCodeService;
use crate::services::certificate_service::CertificateService;
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
                tracing::warn!(
                    "player_from_certificate: legacy cert gamertag={}, effective_game={}",
                    cn,
                    effective_game
                );
                (Some(effective_game), cn.to_string())
            }
        };

        let mut query = player::Entity::find().filter(player::Column::Gamertag.eq(&gamertag));
        if let Some(ref game) = game_filter {
            query = query.filter(player::Column::Game.eq(game));
        }

        match query.one(conn).await {
            Ok(Some(player)) => Ok(player),
            Ok(None) => {
                tracing::warn!(
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
        cert_service: &CertificateService,
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

        // Rotate certificate if expiring or using legacy CN format
        let needs_rotation = actual.is_certificate_expiring().unwrap_or(false)
            || actual.has_legacy_certificate_cn(&game);

        let (certificate, certificate_key) = if needs_rotation {
            match cert_service.sign_player_cert(&gamertag, &game) {
                Ok((cert, key)) => {
                    let cert_pem = cert.pem();
                    let key_pem = key.serialize_pem();

                    let mut cert_active: player::ActiveModel = actual.clone().into();
                    cert_active.certificate = ActiveValue::Set(cert_pem.clone());
                    cert_active.certificate_key = ActiveValue::Set(key_pem.clone());
                    if let Err(e) = cert_active.update(conn).await {
                        tracing::error!("Failed to update rotated certificate: {}", e);
                        (actual.certificate.clone(), actual.certificate_key.clone())
                    } else {
                        tracing::info!("Rotated certificate for player {} at login", gamertag);
                        (cert_pem, key_pem)
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to rotate certificate for {}: {}", gamertag, e);
                    (actual.certificate.clone(), actual.certificate_key.clone())
                }
            }
        } else {
            (actual.certificate.clone(), actual.certificate_key.clone())
        };

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

        let decoded_gamerpic =
            crate::services::GamerpicDecoder::decode(Some(gamerpic)).unwrap_or_default();

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
            certificate,
            certificate_key,
            certificate_ca,
            // The port a client is told to dial has to be one the operator
            // advertises, not the port the socket happens to be bound to. Behind a
            // fronting proxy the two differ, and this value is what a client caches
            // in its keyring.
            config.quic_ports()[0].to_string(),
            server_permissions,
            game,
        ))
    }

    /// Validate a login code, then build a LoginResponse for the resolved player.
    /// Shared by `/api/auth/code` (ncryptf-wrapped) and `/api/auth/code/json` (plain JSON).
    pub async fn login_with_code<C: ConnectionTrait>(
        conn: &C,
        payload: &CodeLoginRequest,
        config: &Server,
        cert_service: &Arc<CertificateService>,
        perm_config_defaults: std::collections::HashMap<String, bool>,
    ) -> Result<LoginResponse, CodeLoginError> {
        let player_record = AuthCodeService::validate_and_consume_code(conn, &payload.code).await?;

        let perm_service = PermissionService::new(perm_config_defaults);
        let response = Self::build_login_response(
            conn,
            config,
            cert_service.as_ref(),
            Some(&perm_service),
            player_record.gamertag.unwrap_or_default(),
            player_record.gamerpic.unwrap_or_default(),
            player_record.game,
        )
        .await?;

        Ok(response)
    }
}
