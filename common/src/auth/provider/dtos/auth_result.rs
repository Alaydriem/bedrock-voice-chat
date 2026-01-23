use serde::{Deserialize, Serialize};

/// Common result from authentication - the authenticated user's identity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResult {
    /// The user's display name / gamertag
    pub gamertag: String,
    /// The user's profile picture URL (base64 encoded)
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