use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::Game;
use crate::structs::permission::PermissionEffect;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct SetPermissionRequest {
    pub gamertag: String,
    pub game: Game,
    pub permission: String,
    pub effect: PermissionEffect,
}
