use std::path::Path;

use async_trait::async_trait;
use common::{structs::config::LoginResponse, Game};
use entity::player;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

use crate::config::ApplicationConfigServer;

/// Result of authentication containing the player info needed for login
pub struct AuthResult {
    pub gamertag: String,
    pub gamerpic: String,
}

/// Trait for authentication providers (Minecraft, Hytale, etc.)
#[async_trait]
pub trait AuthProvider: Send + Sync {
    /// Get the game type for this provider
    fn game_type(&self) -> Game;

    /// Authenticate and return the player's gamertag and gamerpic
    async fn authenticate(&self) -> Result<AuthResult, AuthError>;
}

/// Errors that can occur during authentication
#[derive(Debug)]
pub enum AuthError {
    /// External authentication failed (Xbox Live, Hytale OAuth, etc.)
    ExternalAuthFailed(String),
    /// Player not found in database
    PlayerNotFound,
    /// Player is banished
    PlayerBanished,
    /// Database error
    DatabaseError(String),
    /// Certificate error
    CertificateError(String),
    /// Other errors
    Other(String),
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::ExternalAuthFailed(msg) => write!(f, "External authentication failed: {}", msg),
            AuthError::PlayerNotFound => write!(f, "Player not found in database"),
            AuthError::PlayerBanished => write!(f, "Player is banished"),
            AuthError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            AuthError::CertificateError(msg) => write!(f, "Certificate error: {}", msg),
            AuthError::Other(msg) => write!(f, "Authentication error: {}", msg),
        }
    }
}

impl std::error::Error for AuthError {}

/// Build a LoginResponse for an authenticated player
/// This is the shared logic used by all auth providers
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
    let certificate_ca = std::fs::read_to_string(Path::new(&format!(
        "{}/ca.crt",
        config.tls.certs_path
    )))
    .map_err(|e| {
        tracing::error!("Failed to read CA certificate: {}", e);
        AuthError::CertificateError(e.to_string())
    })?;

    Ok(LoginResponse {
        gamertag,
        gamerpic,
        keypair: common::structs::config::Keypair {
            pk: kp.get_public_key(),
            sk: kp.get_public_key(),
        },
        signature: common::structs::config::Keypair {
            pk: sp.get_public_key(),
            sk: sp.get_public_key(),
        },
        certificate: actual.certificate,
        certificate_key: actual.certificate_key,
        certificate_ca,
        quic_connect_string: config.quic_port.to_string(),
    })
}
