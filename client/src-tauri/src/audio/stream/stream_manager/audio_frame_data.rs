#[derive(Debug, Clone)]
pub(crate) struct AudioFrameData<T> {
    pub pcm: Vec<T>,
    /// Timestamp when audio was captured at the CPAL callback, for accurate recording timecode
    pub captured_at_ms: u64,
}
