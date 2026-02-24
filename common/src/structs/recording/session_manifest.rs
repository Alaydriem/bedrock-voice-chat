use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct SessionManifest {
    pub session_id: String,
    pub start_timestamp: u64,
    pub end_timestamp: Option<u64>,
    pub duration_ms: Option<u64>,
    pub emitter_player: String,
    pub participants: Vec<String>,
    pub created_at: String,
}
