/// Generates Opus-encoded silence packets
pub struct SilenceEncoder {
    encoder: opus2::Encoder,
    channels: u16,
    frame_size: usize,
}

impl SilenceEncoder {
    pub fn new(sample_rate: u32, channels: u16) -> Result<Self, anyhow::Error> {
        let channels_enum = if channels == 1 {
            opus2::Channels::Mono
        } else {
            opus2::Channels::Stereo
        };

        let mut encoder =
            opus2::Encoder::new(sample_rate, channels_enum, opus2::Application::Audio)?;
        encoder.set_bitrate(opus2::Bitrate::Bits(64000))?;

        // 20ms frame size
        let frame_size = (sample_rate as usize * 20) / 1000;

        Ok(Self {
            encoder,
            channels,
            frame_size,
        })
    }

    /// Generate Opus-encoded silence packets to fill a gap
    pub fn encode_silence(
        &mut self,
        total_samples: u32,
    ) -> Result<Vec<(Vec<u8>, u32)>, anyhow::Error> {
        let mut packets = Vec::new();
        let mut remaining = total_samples as usize;

        let channels = self.channels as usize;
        let silence = vec![0.0f32; self.frame_size * channels];
        let mut opus_out = vec![0u8; 4000];

        while remaining >= self.frame_size {
            let encoded_len = self.encoder.encode_float(&silence, &mut opus_out)?;
            packets.push((opus_out[..encoded_len].to_vec(), self.frame_size as u32));
            remaining -= self.frame_size;
        }

        Ok(packets)
    }
}
