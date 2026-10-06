use serde::{Deserialize, Serialize};

use crate::Game;
use crate::structs::permission::AdminAction;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminPermissionRequest {
    pub gamertag: String,
    pub game: Game,
    pub action: AdminAction,
}
