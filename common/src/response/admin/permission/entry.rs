use serde::{Deserialize, Serialize};

use crate::structs::permission::PermissionEffect;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct PermissionEntry {
    pub permission: String,
    pub effect: PermissionEffect,
}
