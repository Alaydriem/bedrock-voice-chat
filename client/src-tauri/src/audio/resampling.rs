pub struct AudioResampling;

impl AudioResampling {
    /// Native sample rate for Opus encoding
    pub const OPUS_SAMPLE_RATE: u32 = 48000;

    /// Check if resampling is needed (any rate != 48 kHz)
    #[allow(dead_code)]
    pub fn needs_resampling(device_sample_rate: u32) -> bool {
        device_sample_rate != Self::OPUS_SAMPLE_RATE
    }
}
