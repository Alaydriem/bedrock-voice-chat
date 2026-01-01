use crate::audio::stream::stream_manager::AudioSinkType;
use base64::{engine::general_purpose, Engine as _};
use common::RecordingPlayerData as PlayerData;

pub mod adaptive;
pub mod audio_processor;
mod jitter_buffer;
pub mod jitter_buffer_source;
pub mod metrics;

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
    pub sample_rate: u32,
    pub data: Vec<u8>,
    pub route: AudioSinkType,
    pub emitter: PlayerData,
    pub listener: PlayerData,
    pub buffer_size_ms: u32,
    pub time_between_reports_secs: u64,
}

impl EncodedAudioFramePacket {
    pub fn get_author(&self) -> String {
        if let Some(ref client_id) = self.emitter.client_id {
            return general_purpose::STANDARD.encode(client_id);
        }
        String::from("")
    }

    pub fn get_client_id(&self) -> Vec<u8> {
        self.emitter.client_id.clone().unwrap_or_default()
    }
}
