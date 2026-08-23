/// Decoded audio frame with metadata
#[derive(Debug)]
pub struct DecodedAudioFrame {
    pub pcm_data: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
    pub relative_timestamp_ms: u64,
}
