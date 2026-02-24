#[cfg(feature = "audio")]
pub const BUFFER_SIZE: u32 = 960;

/// Supported sample rates in order of preference (highest first)
#[cfg(feature = "audio")]
pub const SUPPORTED_SAMPLE_RATES: [u32; 2] = [48000, 44100];
