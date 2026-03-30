use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::super::auth_status::HytaleAuthStatus;
use crate::response::LoginResponse;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct HytaleDeviceFlowStatusResponse {
    pub status: HytaleAuthStatus,
    pub login_response: Option<LoginResponse>,
}
