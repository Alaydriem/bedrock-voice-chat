use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Response from starting audio playback.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct AudioEventResponse {
    pub event_id: String,
    pub duration_ms: u64,
}
