/// Chunk of PCM audio data
#[derive(Debug)]
pub enum PcmChunk {
    /// Decoded audio samples (f32)
    Audio(Vec<f32>),
    /// Silence to fill gaps (sample count)
    Silence(usize),
}
