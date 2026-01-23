//! Device code response from Hytale OAuth API

use serde::Deserialize;

/// Response from the device authorization endpoint
#[derive(Deserialize)]
pub(crate) struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    #[serde(default)]
    pub verification_uri_complete: Option<String>,
    pub expires_in: u64,
    pub interval: u64,
}
