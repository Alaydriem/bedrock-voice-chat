mod display_claims;
mod xui;

pub(crate) use display_claims::DisplayClaims;
pub(crate) use xui::Xui;

use serde::Deserialize;

/// Response from Xbox Live authentication endpoints
#[derive(Deserialize)]
pub(crate) struct XboxAuthResponse {
    #[serde(rename = "Token")]
    pub token: String,
    #[serde(rename = "DisplayClaims")]
    pub display_claims: DisplayClaims,
}
