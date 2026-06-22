use serde::{Deserialize, Serialize};

use crate::Game;
use crate::structs::permission::PermissionEffect;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct SetPermissionRequest {
    pub gamertag: String,
    pub game: Game,
    pub permission: String,
    pub effect: PermissionEffect,
}
