use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Common result from authentication - the authenticated user's identity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResult {
    /// The user's display name / gamertag
    pub gamertag: String,
    /// The user's profile picture (base64 encoded for Minecraft, empty for Hytale)
    pub gamerpic: String,
}

impl AuthResult {
    /// Create a new AuthResult
    pub fn new(gamertag: String, gamerpic: String) -> Self {
        Self { gamertag, gamerpic }
    }

    /// Create an AuthResult with no profile picture
    pub fn without_gamerpic(gamertag: String) -> Self {
        Self {
            gamertag,
            gamerpic: String::new(),
        }
    }
}

/// Errors that can occur during authentication
#[derive(Debug, Error)]
pub enum AuthError {
    /// Network/HTTP request failed
    #[error("Network error: {0}")]
    Network(String),

    /// Authentication was rejected by the provider
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    /// User profile was not found after successful auth
    #[error("Profile not found")]
    ProfileNotFound,

    /// Response from provider was malformed or unexpected
    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    /// Underlying HTTP client error
    #[error("Request error: {0}")]
    Request(#[from] reqwest::Error),
}
