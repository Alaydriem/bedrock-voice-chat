use crate::audio::recording::renderer::stream::opus::OpusStreamInfo;

/// Session metadata for the user data box
#[derive(Debug, Clone)]
pub struct SessionMetadata {
    /// Unique session identifier
    pub session_id: String,
    /// Unix timestamp in milliseconds when the session started
    pub start_timestamp: u64,
    /// Name of the player this audio belongs to
    pub player_name: String,
    /// Duration of the recording in milliseconds (if known)
    pub duration_ms: Option<u64>,
}

impl SessionMetadata {
    /// Create session metadata from OpusStreamInfo
    pub fn from_stream_info(info: &OpusStreamInfo, duration_ms: Option<u64>) -> Self {
        Self {
            session_id: info.session_info.session_id.clone(),
            start_timestamp: info.session_info.start_timestamp,
            player_name: info.session_info.player_name.clone(),
            duration_ms,
        }
    }
}
