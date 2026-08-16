use crate::audio::recording::renderer::SessionInfo;

/// Audio metadata from the stream
#[derive(Debug, Clone)]
pub struct PcmStreamInfo {
    pub sample_rate: u32,
    pub channels: u16,
    pub first_frame_timestamp_ms: u64,
    pub session_info: SessionInfo,
}
