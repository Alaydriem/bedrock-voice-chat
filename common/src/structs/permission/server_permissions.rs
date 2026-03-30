use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::Permission;

#[derive(Debug, Clone, Serialize, Deserialize, Default, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ServerPermissions {
    pub allowed: Vec<Permission>,
}
