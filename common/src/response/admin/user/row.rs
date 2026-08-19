use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::Game;
use crate::structs::permission::Permission;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct AdminUserRow {
    pub gamertag: String,
    pub game: Game,
    pub banished: bool,
    // Whether this identity holds a live voice connection right now. Read from the
    // connection registry, not from the database.
    pub connected: bool,
    // The effective set: server defaults with this player's overrides applied. The
    // override list, which separates an explicit allow from a default allow, is what the
    // permission editor fetches separately.
    pub permissions: Vec<Permission>,
    pub created_at: i64,
}
