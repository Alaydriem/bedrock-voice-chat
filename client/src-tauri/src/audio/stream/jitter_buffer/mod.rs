use crate::audio::stream::stream_manager::AudioSinkType;
use common::RecordingPlayerData as PlayerData;

pub mod adaptive;
pub mod audio_processor;
pub mod metrics;

mod admission;
mod buffer;
mod handle;
mod pan_state;
mod ring_entry;
mod source;
mod source_error;
mod warmup_gate;

pub use admission::{Admission, FrameAdmission};
pub use buffer::JitterBuffer;
pub use handle::JitterBufferHandle;
pub use pan_state::PanState;
pub use source_error::JitterBufferError;
pub use warmup_gate::WarmupGate;

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct EncodedAudioFramePacket {
    pub(crate) timestamp: u64,
    pub(crate) sample_rate: u32,
    // `Bytes` because the router clones a whole packet per frame to hand it to playback, and the
    // Opus payload is the only part of it worth copying.
    pub(crate) data: bytes::Bytes,
    pub(crate) route: AudioSinkType,
    pub(crate) emitter: PlayerData,
    pub(crate) listener: PlayerData,
    pub(crate) buffer_size_ms: u32,
    pub(crate) time_between_reports_secs: u64,
}

impl EncodedAudioFramePacket {
    // Every packet is built with these two values.
    const BUFFER_SIZE_MS: u32 = 120;
    const TIME_BETWEEN_REPORTS_SECS: u64 = 30;

    pub fn new(
        timestamp: u64,
        sample_rate: u32,
        data: bytes::Bytes,
        spatial: bool,
        emitter: PlayerData,
        listener: PlayerData,
    ) -> Self {
        Self {
            timestamp,
            sample_rate,
            data,
            route: AudioSinkType::from_spatial(spatial),
            emitter,
            listener,
            buffer_size_ms: Self::BUFFER_SIZE_MS,
            time_between_reports_secs: Self::TIME_BETWEEN_REPORTS_SECS,
        }
    }

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
