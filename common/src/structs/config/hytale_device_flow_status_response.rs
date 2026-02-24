use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::hytale_auth_status::HytaleAuthStatus;
use super::login_response::LoginResponse;

/// Response when polling Hytale device flow status
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct HytaleDeviceFlowStatusResponse {
    pub status: HytaleAuthStatus,
    pub login_response: Option<LoginResponse>,
}
