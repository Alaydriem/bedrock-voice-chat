use serde::{Deserialize, Serialize};

use super::player_metadata::PlayerMetadata;

/// Concrete header type for input recording WAL entries
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InputRecordingHeader {
    pub sample_rate: u32,
    pub channels: u16,
    pub relative_timestamp_ms: Option<u64>,
    pub emitter_metadata: PlayerMetadata,
}
