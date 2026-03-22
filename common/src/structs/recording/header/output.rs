use serde::{Deserialize, Serialize};

use crate::structs::recording::player::PlayerMetadata;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OutputRecordingHeader {
    pub sample_rate: u32,
    pub channels: u16,
    pub relative_timestamp_ms: u64,
    pub emitter_metadata: PlayerMetadata,
    pub listener_metadata: PlayerMetadata,
    pub is_spatial: bool,
}
