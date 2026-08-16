#[derive(Debug)]
pub enum AudioProcessorError {
    DecoderError(opus2::Error),
    RingBufferFull,
}

impl std::fmt::Display for AudioProcessorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioProcessorError::DecoderError(e) => write!(f, "Opus decoder error: {:?}", e),
            AudioProcessorError::RingBufferFull => write!(f, "Ring buffer is full"),
        }
    }
}

impl std::error::Error for AudioProcessorError {}
