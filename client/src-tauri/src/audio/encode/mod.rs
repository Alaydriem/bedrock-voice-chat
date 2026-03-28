pub(crate) mod encoder;
pub(crate) mod ogg_writer;

pub use encoder::AudioFileEncoder;

pub struct EncodeOutput {
    pub opus_bytes: Vec<u8>,
    pub duration_ms: u64,
    pub original_filename: String,
}
