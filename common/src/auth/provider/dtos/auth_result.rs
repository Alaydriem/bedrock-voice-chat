use serde::{Deserialize, Serialize};

/// Common result from authentication - the authenticated user's identity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResult {
    /// The user's display name / gamertag
    pub gamertag: String,
    /// The user's profile picture URL (base64 encoded)
    pub gamerpic: String,
    /// The user's Minecraft Java username (if they own Java Edition)
    #[serde(default)]
    pub minecraft_username: Option<String>,
}

impl AuthResult {
    /// Create a new AuthResult
    pub fn new(gamertag: String, gamerpic: String) -> Self {
        Self {
            gamertag,
            gamerpic,
            minecraft_username: None,
        }
    }

    /// Create an AuthResult with no profile picture
    pub fn without_gamerpic(gamertag: String) -> Self {
        Self {
            gamertag,
            gamerpic: String::new(),
            minecraft_username: None,
        }
    }

    /// Set the Minecraft Java username
    pub fn with_minecraft_username(mut self, username: Option<String>) -> Self {
        self.minecraft_username = username;
        self
    }
}