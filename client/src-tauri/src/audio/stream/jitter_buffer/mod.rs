use crate::audio::stream::stream_manager::AudioSinkType;
use common::RecordingPlayerData as PlayerData;

pub mod adaptive;
pub mod audio_processor;
pub mod metrics;

mod buffer;
mod handle;
mod pan_state;
mod pending_recording;
mod seq_clock;
mod source;
mod source_error;

pub use buffer::JitterBuffer;
pub use handle::JitterBufferHandle;
pub use pan_state::PanState;

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
