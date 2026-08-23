use super::audio_processor::AudioProcessorError;

#[derive(Debug)]
pub enum JitterBufferError {
    AudioProcessorError(AudioProcessorError),
    InvalidPacket,
}

impl std::fmt::Display for JitterBufferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JitterBufferError::AudioProcessorError(e) => write!(f, "Audio processor error: {}", e),
            JitterBufferError::InvalidPacket => write!(f, "Invalid packet data"),
        }
    }
}

impl std::error::Error for JitterBufferError {}

impl From<AudioProcessorError> for JitterBufferError {
    fn from(err: AudioProcessorError) -> Self {
        JitterBufferError::AudioProcessorError(err)
    }
}
