use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct BedrockLogEntry {
    pub timestamp_ms: i64,
    pub level: String,
    pub target: String,
    pub message: String,
}
