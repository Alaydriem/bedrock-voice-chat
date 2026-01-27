//! Microsoft OAuth access token response

use serde::Deserialize;

/// Response from Microsoft OAuth token endpoint
#[derive(Deserialize)]
pub(crate) struct AccessTokenResponse {
    pub access_token: String,
}
