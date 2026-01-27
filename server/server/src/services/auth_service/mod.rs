//! Authentication service for building login responses

use std::path::Path;

use common::{
    structs::config::{Keypair, LoginResponse},
    Game,
};
use entity::player;
use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

use crate::config::ApplicationConfigServer;

/// Errors that can occur during authentication
#[derive(Debug)]
pub enum AuthError {
    /// Player not found in database
    PlayerNotFound,
    /// Player is banished
    PlayerBanished,
    /// Database error
    DatabaseError(String),
    /// Certificate error
    CertificateError(String),
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::PlayerNotFound => write!(f, "Player not found in database"),
            AuthError::PlayerBanished => write!(f, "Player is banished"),
            AuthError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            AuthError::CertificateError(msg) => write!(f, "Certificate error: {}", msg),
        }
    }
}

impl std::error::Error for AuthError {}

/// Service for authentication operations
pub struct AuthService;

impl AuthService {
    /// Build a LoginResponse for an authenticated player
    ///
    /// # Arguments
    /// * `conn` - Database connection
    /// * `config` - Server configuration
    /// * `gamertag` - Player's gamertag
    /// * `gamerpic` - Player's profile picture (base64 encoded URL)
    /// * `game` - Game type (Minecraft, Hytale)
    pub async fn build_login_response<C: ConnectionTrait>(
        conn: &C,
        config: &ApplicationConfigServer,
        gamertag: String,
        gamerpic: String,
        game: Game,
    ) -> Result<LoginResponse, AuthError> {
        // Look up player by gamertag and game type
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

        // Update gamerpic in database if it has changed
        if actual.gamerpic.as_ref() != Some(&gamerpic) {
            let mut player_active: player::ActiveModel = actual.clone().into();
            player_active.gamerpic = ActiveValue::Set(Some(gamerpic.clone()));
            player_active.update(conn).await.map_err(|e| {
                tracing::error!("Failed to update gamerpic: {}", e);
                AuthError::DatabaseError(e.to_string())
            })?;
            tracing::debug!("Updated gamerpic for player {}", gamertag);
        }

        // Block banished users
        if actual.banished {
            tracing::info!("Player {} is banished", gamertag);
            return Err(AuthError::PlayerBanished);
        }

        // Get keypair
        let kp = actual.get_keypair().map_err(|e| {
            tracing::error!("Failed to get keypair: {}", e);
            AuthError::CertificateError(e.to_string())
        })?;

        // Get signature
        let sp = actual.get_signature().map_err(|e| {
            tracing::error!("Failed to get signature: {}", e);
            AuthError::CertificateError(e.to_string())
        })?;

        // Read CA certificate
        let certificate_ca =
            std::fs::read_to_string(Path::new(&format!("{}/ca.crt", config.tls.certs_path)))
                .map_err(|e| {
                    tracing::error!("Failed to read CA certificate: {}", e);
                    AuthError::CertificateError(e.to_string())
                })?;

        Ok(LoginResponse::new(
            gamertag,
            gamerpic,
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
        ))
    }
}
