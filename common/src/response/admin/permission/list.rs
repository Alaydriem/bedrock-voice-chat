use serde::{Deserialize, Serialize};

use crate::response::admin::permission::PermissionEntry;
use crate::Game;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct PermissionListResponse {
    pub gamertag: String,
    pub game: Game,
    pub entries: Vec<PermissionEntry>,
}
