use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
#[serde(tag = "status")]
pub enum ConnectionHealth {
    Connected,
    Reconnecting { attempt: u32 },
    Disconnected,
    Failed,
}
