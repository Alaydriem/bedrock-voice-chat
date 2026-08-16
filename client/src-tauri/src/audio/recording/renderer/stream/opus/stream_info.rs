use crate::audio::recording::renderer::SessionInfo;

/// Stream metadata extracted from first packet
#[derive(Debug, Clone)]
pub struct OpusStreamInfo {
    pub sample_rate: u32,
    pub channels: u16,
    pub first_packet_timestamp_ms: u64,
    pub session_info: SessionInfo,
}
