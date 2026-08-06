use crate::audio::stream::stream_manager::AudioSinkType;
use common::RecordingPlayerData as PlayerData;

pub mod adaptive;
pub mod audio_processor;
mod jitter_buffer;
pub mod jitter_buffer_source;
pub mod metrics;
pub mod pan_state;

pub use jitter_buffer::{JitterBuffer, JitterBufferHandle};
pub use pan_state::PanState;

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
    /// The key this frame's sink and gain are resolved under.
    ///
    /// The emitter's device id, so one player speaking from two devices gets two sinks
    /// rather than two streams interleaved into one. A synthetic emitter — jukebox
    /// playback, channel API audio — has no connection and falls back to its name, which
    /// already carries a per-event suffix and so keeps concurrent playbacks apart.
    pub fn sink_key(&self) -> String {
        match self.emitter.device {
            Some(device) => device.to_string(),
            None => self.emitter.name.clone(),
        }
    }
}
