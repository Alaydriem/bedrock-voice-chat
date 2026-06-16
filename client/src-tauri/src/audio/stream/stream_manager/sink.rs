// Push-based sink for post-mix output samples. Enum dispatch, no trait objects.
pub(crate) enum AudioOutputSink {
    Rodio,
    #[cfg(feature = "e2e")]
    Fake(CapturingSink),
}

#[cfg(feature = "e2e")]
pub struct CapturingSink {
    tx: flume::Sender<Vec<f32>>,
    sample_rate: u32,
    channels: u16,
}

#[cfg(feature = "e2e")]
impl CapturingSink {
    pub fn new(tx: flume::Sender<Vec<f32>>, sample_rate: u32, channels: u16) -> Self {
        Self {
            tx,
            sample_rate,
            channels,
        }
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn channels(&self) -> u16 {
        self.channels
    }

    // Push captured post-mix samples out to the orchestrator.
    pub fn submit(&self, pcm: Vec<f32>) {
        let _ = self.tx.send(pcm);
    }
}

#[cfg(all(test, feature = "e2e"))]
mod tests {
    use super::*;
    use rodio::Source;

    fn sine_440(channels: u16, sample_rate: u32, frames: usize) -> Vec<f32> {
        let freq = 440.0_f32;
        let sr = sample_rate as f32;
        let mut out = Vec::with_capacity(frames * channels as usize);
        for n in 0..frames {
            let s = (2.0 * std::f32::consts::PI * freq * n as f32 / sr).sin() * 0.5;
            for _ in 0..channels {
                out.push(s);
            }
        }
        out
    }

    fn rms(samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
        (sum_sq / samples.len() as f32).sqrt()
    }

    // Validates the mixer -> drain -> capture mechanism the Fake output arm
    // relies on: a finite signal added to the mixer must surface as non-empty
    // PCM with non-zero energy on the CapturingSink's channel. Full-stream
    // wiring (decode/jitter/spatial through SinkManager) is validated by the
    // scenario integration tests.
    #[test]
    fn mixer_drain_captures_injected_signal() {
        let channels = std::num::NonZeroU16::new(2).unwrap();
        let sample_rate = std::num::NonZeroU32::new(48_000).unwrap();

        let (mix, mut source) = rodio::mixer::mixer(channels, sample_rate);

        // Keep the mixer alive when it has no active sources.
        mix.add(rodio::source::Zero::new(channels, sample_rate));

        // 100ms of 440Hz at 48kHz stereo.
        let frames = 4_800usize;
        let signal = sine_440(channels.get(), sample_rate.get(), frames);
        mix.add(rodio::buffer::SamplesBuffer::new(
            channels,
            sample_rate,
            signal,
        ));

        let (tx, rx) = flume::unbounded::<Vec<f32>>();
        let cap = CapturingSink::new(tx, sample_rate.get(), channels.get());

        // Drain ~120ms worth of samples (covers the 100ms injected buffer).
        let block_len = (sample_rate.get() / 50) as usize * channels.get() as usize;
        let blocks = 6;
        for _ in 0..blocks {
            let mut block = Vec::with_capacity(block_len);
            for _ in 0..block_len {
                match source.next() {
                    Some(s) => block.push(s),
                    None => break,
                }
            }
            cap.submit(block);
        }

        let captured: Vec<f32> = rx.drain().flatten().collect();
        assert!(!captured.is_empty(), "expected captured PCM, got empty");
        assert!(
            rms(&captured) > 0.0,
            "expected non-zero RMS from injected 440Hz signal, got {}",
            rms(&captured)
        );
    }
}
