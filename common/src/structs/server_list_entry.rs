use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ServerListEntry {
    pub server: String,
    pub player: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game: Option<String>,
}
