use serde::Deserialize;

/// Successful token response
#[derive(Deserialize)]
pub(crate) struct TokenResponse {
    pub access_token: String,
}
