use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Response when starting a Hytale device flow
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct HytaleDeviceFlowStartResponse {
    pub session_id: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: String,
    pub expires_in: u32,
    pub interval: u32,
}
