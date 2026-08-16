/// Chunk of raw Opus data for lossless muxing
#[derive(Debug)]
pub enum OpusChunk {
    /// Raw Opus packet from WAL
    Packet {
        data: Vec<u8>,
        duration_samples: u32,
    },
    /// Encoded silence to fill gaps
    Silence {
        data: Vec<u8>,
        duration_samples: u32,
    },
}
