use crate::audio::stream::stream_manager::AudioSinkType;
use base64::{engine::general_purpose, Engine as _};
use common::{structs::packet::PacketOwner, Coordinate, Dimension, Orientation};

mod jitter_buffer;
pub use jitter_buffer::{JitterBuffer, JitterBufferHandle, SpatialAudioData};

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct SeqClock {
    base_ms: i64,
    last_idx: i64,
    inited: bool,
}

#[allow(dead_code)]
impl SeqClock {
    fn new() -> Self {
        Self {
            base_ms: 0,
            last_idx: -1,
            inited: false,
        }
    }

    fn map_ts_to_index(&mut self, ts_ms: i64) -> (i64 /*idx*/, i64 /*missing*/) {
        const FRAME_MS: i64 = 20;

        if !self.inited {
            self.base_ms = ts_ms - (ts_ms % FRAME_MS);
            self.last_idx = (ts_ms - self.base_ms + FRAME_MS / 2) / FRAME_MS; // ~0
            self.inited = true;
            return (self.last_idx, 0);
        }

        let mut idx = (ts_ms - self.base_ms + FRAME_MS / 2) / FRAME_MS;

        // Optional small resync:
        let frame_center = self.base_ms + idx * FRAME_MS;
        let err = ts_ms - frame_center;
        if err.abs() > 8 {
            // ~ 40% of frame; tune as you like
            self.base_ms += err;
            idx = (ts_ms - self.base_ms + FRAME_MS / 2) / FRAME_MS;
        }

        let expected = self.last_idx + 1;
        let missing = (idx - expected).max(0);

        self.last_idx = idx;
        (idx, missing)
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct EncodedAudioFramePacket {
    pub timestamp: u64,
    #[allow(dead_code)]
    pub sample_rate: u32,
    pub data: Vec<u8>, // Opus-encoded data
    #[allow(dead_code)]
    pub route: AudioSinkType,
    pub coordinate: Option<Coordinate>,
    #[allow(dead_code)]
    pub orientation: Option<Orientation>,
    #[allow(dead_code)]
    pub dimension: Option<Dimension>,
    pub spatial: Option<bool>,
    pub owner: Option<PacketOwner>,
}

impl EncodedAudioFramePacket {
    pub fn get_author(&self) -> String {
        match &self.owner {
            Some(owner) => {
                // Utilize the client ID so that the same author can receive and hear multiple incoming
                // network streams. Without this, the audio packets for the same author across two streams
                // come in sequence and playback sounds corrupted
                return general_purpose::STANDARD.encode(&owner.client_id);
            }
            None => String::from(""),
        }
    }

    pub fn get_client_id(&self) -> Vec<u8> {
        match &self.owner {
            Some(owner) => owner.client_id.clone(),
            None => vec![],
        }
    }
}
