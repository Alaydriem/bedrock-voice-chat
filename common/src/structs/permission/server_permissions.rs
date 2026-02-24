use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::permission::Permission;

/// Server permissions returned in LoginResponse and the state refresh endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, Default, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ServerPermissions {
    pub allowed: Vec<Permission>,
}
