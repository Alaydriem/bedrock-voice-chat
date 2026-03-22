use serde::Deserialize;

use super::hytale_profile::HytaleProfile;

/// Response from the profile endpoint
#[derive(Deserialize)]
pub(crate) struct HytaleProfileResponse {
    pub profiles: Vec<HytaleProfile>,
}
