//! Xbox Live authentication response DTOs

use serde::Deserialize;

/// Response from Xbox Live authentication endpoints
#[derive(Deserialize)]
pub(crate) struct XboxAuthResponse {
    #[serde(rename = "Token")]
    pub token: String,
    #[serde(rename = "DisplayClaims")]
    pub display_claims: DisplayClaims,
}

/// Display claims containing user info
#[derive(Deserialize)]
pub(crate) struct DisplayClaims {
    pub xui: Vec<Xui>,
}

/// Xbox User Info
#[derive(Deserialize)]
pub(crate) struct Xui {
    #[serde(rename = "uhs")]
    pub user_hash: String,
}
