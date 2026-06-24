use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct DiscordLinkStatus {
    pub configured: bool,
    pub linked: bool,
    pub role_count: u32,
    pub last_synced: Option<i64>,
    pub expired: bool,
}
