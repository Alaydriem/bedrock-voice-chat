use crate::audio::types::OPUS_SAMPLE_RATE;
use rubato::{FftFixedIn, Resampler};

/// Audio resampler for converting any sample rate to 48 kHz (Opus native rate)
pub struct AudioResampler {
    resampler: FftFixedIn<f32>,
    input_buffer: Vec<f32>,
    input_frames_needed: usize,
    source_rate: u32,
}

impl AudioResampler {
    /// Create resampler for source_rate → 48 kHz conversion
    /// Returns None if source rate is already 48 kHz (no resampling needed)
    pub fn new_if_needed(source_rate: u32) -> Option<Result<Self, anyhow::Error>> {
        if source_rate == OPUS_SAMPLE_RATE {
            return None;
        }

        // Chunk size = 20ms of audio at source rate
        let chunk_size = (source_rate / 50) as usize;

        // Lower sub_chunks on mobile for performance
        #[cfg(any(target_os = "android", target_os = "ios"))]
        let sub_chunks = 1;

        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        let sub_chunks = 2;

        let resampler = match FftFixedIn::<f32>::new(
            source_rate as usize,
            OPUS_SAMPLE_RATE as usize,
            chunk_size,
            sub_chunks,
            1, // mono
        ) {
            Ok(r) => r,
            Err(e) => return Some(Err(anyhow::anyhow!("Failed to create resampler: {:?}", e))),
        };

        let input_frames_needed = resampler.input_frames_next();

        Some(Ok(Self {
            resampler,
            input_buffer: Vec::with_capacity(chunk_size * 2),
            input_frames_needed,
            source_rate,
        }))
    }

    /// Returns the source sample rate this resampler was created for
    #[allow(dead_code)]
    pub fn source_rate(&self) -> u32 {
        self.source_rate
    }

    /// Process mono PCM samples, returns resampled output at 48 kHz
    pub fn process(&mut self, input: &[f32]) -> Vec<f32> {
        self.input_buffer.extend_from_slice(input);
        let mut output = Vec::new();

        while self.input_buffer.len() >= self.input_frames_needed {
            let chunk: Vec<f32> = self.input_buffer.drain(..self.input_frames_needed).collect();

            if let Ok(resampled) = self.resampler.process(&[chunk.as_slice()], None) {
                if !resampled.is_empty() && !resampled[0].is_empty() {
                    output.extend_from_slice(&resampled[0]);
                }
            }

            self.input_frames_needed = self.resampler.input_frames_next();
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_resampler_for_48khz() {
        let result = AudioResampler::new_if_needed(48000);
        assert!(result.is_none());
    }

    #[test]
    fn test_resampler_created_for_44100() {
        let result = AudioResampler::new_if_needed(44100);
        assert!(result.is_some());
        let resampler = result.unwrap().unwrap();
        assert_eq!(resampler.source_rate(), 44100);
    }

    #[test]
    fn test_resampler_created_for_16000() {
        let result = AudioResampler::new_if_needed(16000);
        assert!(result.is_some());
        let resampler = result.unwrap().unwrap();
        assert_eq!(resampler.source_rate(), 16000);
    }

    #[test]
    fn test_resampling_44100_output_count() {
        let mut resampler = AudioResampler::new_if_needed(44100).unwrap().unwrap();

        // 882 samples at 44.1 kHz (20ms) should produce ~960 samples at 48 kHz
        let input: Vec<f32> = vec![0.5; 882];
        let output = resampler.process(&input);

        // Output should be approximately 960 samples (882 * 48000/44100 ≈ 960)
        // Allow some tolerance for resampler buffering
        assert!(
            output.len() >= 900 && output.len() <= 1000,
            "Expected ~960 samples, got {}",
            output.len()
        );
    }

    #[test]
    fn test_resampling_16000_output_count() {
        let mut resampler = AudioResampler::new_if_needed(16000).unwrap().unwrap();

        // 320 samples at 16 kHz (20ms) should produce ~960 samples at 48 kHz
        let input: Vec<f32> = vec![0.5; 320];
        let output = resampler.process(&input);

        // Output should be approximately 960 samples (320 * 48000/16000 = 960)
        // Allow some tolerance for resampler buffering
        assert!(
            output.len() >= 900 && output.len() <= 1000,
            "Expected ~960 samples, got {}",
            output.len()
        );
    }
}
