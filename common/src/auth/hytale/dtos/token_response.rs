//! Token response from Hytale OAuth API

use serde::Deserialize;

/// Successful token response
#[derive(Deserialize)]
pub(crate) struct TokenResponse {
    pub access_token: String,
}

/// Error response from token endpoint
#[derive(Deserialize)]
pub(crate) struct TokenErrorResponse {
    pub error: String,
    #[serde(default)]
    pub error_description: Option<String>,
}
