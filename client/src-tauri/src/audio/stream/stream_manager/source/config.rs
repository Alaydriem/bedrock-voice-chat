use rodio::cpal::SampleFormat;

// Resolved capture parameters shared by every input source variant. The gate,
// resampler, and InputProcessCore are built from these regardless of whether the
// samples originate from a live cpal device or the test bridge, so the input and
// QUIC-sender paths see one uniform configuration.
pub(crate) struct CaptureConfig {
    pub sample_rate: u32,
    pub channels: u16,
    pub sample_format: SampleFormat,
    pub buffer_size: rodio::cpal::BufferSize,
}
