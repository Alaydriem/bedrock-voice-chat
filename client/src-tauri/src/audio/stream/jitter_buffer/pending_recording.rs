use common::RecordingPlayerData;

/// Recording data waiting to be emitted when its corresponding audio samples are consumed
#[derive(Clone)]
pub(super) struct PendingRecording {
    pub(super) opus_data: Vec<u8>,
    pub(super) emitter: RecordingPlayerData,
    pub(super) listener: RecordingPlayerData,
    pub(super) sample_rate: u32,
    pub(super) is_spatial: bool,
    pub(super) samples_remaining: usize,
    pub(super) captured_timestamp_ms: u64,
}
