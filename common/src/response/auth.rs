use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::structs::permission::ServerPermissions;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct AuthStateResponse {
    pub server_permissions: ServerPermissions,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub certificate: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub certificate_key: Option<String>,
}
