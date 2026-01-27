//! Profile response from Hytale API

use serde::Deserialize;

/// Response from the profile endpoint
#[derive(Deserialize)]
pub(crate) struct HytaleProfileResponse {
    pub profiles: Vec<HytaleProfile>,
}

/// Individual Hytale player profile
#[derive(Deserialize)]
pub(crate) struct HytaleProfile {
    pub uuid: String,
    pub username: String,
}
