use serde::{Deserialize, Serialize};

use super::player_metadata::PlayerMetadata;

/// Concrete header type for output recording WAL entries
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OutputRecordingHeader {
    pub sample_rate: u32,
    pub channels: u16,
    pub relative_timestamp_ms: u64,
    pub emitter_metadata: PlayerMetadata,
    pub listener_metadata: PlayerMetadata,
    pub is_spatial: bool,
}
