use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::structs::permission::ServerPermissions;

/// Response from the auth state endpoint.
/// May include re-issued certificate data if the server detected an old CN format or expiry.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct AuthStateResponse {
    pub server_permissions: ServerPermissions,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub certificate: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub certificate_key: Option<String>,
}
