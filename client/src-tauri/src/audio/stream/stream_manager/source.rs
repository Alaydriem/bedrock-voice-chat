// Pull-based source of raw f32 frames fed through the real input processing
// path. Enum dispatch, no trait objects.
pub(crate) enum AudioInputSource {
    Cpal,
    #[cfg(feature = "e2e")]
    Fake(BridgeInputSource),
}

#[cfg(feature = "e2e")]
const FRAME_SAMPLES: usize = 960;

#[cfg(feature = "e2e")]
const FRAME_INTERVAL_MS: f64 = 20.0;

#[cfg(feature = "e2e")]
use super::frame_clock::FrameClock;

// Pull-based fake microphone. The orchestrator fills the bridge receiver with
// variable-size PCM chunks; this source accumulates them and clocks out exactly
// 960-sample (20 ms @ 48 kHz) mono frames on a FrameClock cadence so the real
// input pipeline and QUIC send path see real-time-paced input rather than one
// burst that overruns the bounded datagram queue.
#[cfg(feature = "e2e")]
pub struct BridgeInputSource {
    rx: flume::Receiver<Vec<f32>>,
    sample_rate: u32,
    channels: u16,
    pending: std::collections::VecDeque<f32>,
    clock: Option<FrameClock>,
}

#[cfg(feature = "e2e")]
impl BridgeInputSource {
    pub fn new(rx: flume::Receiver<Vec<f32>>, sample_rate: u32, channels: u16) -> Self {
        Self {
            rx,
            sample_rate,
            channels,
            pending: std::collections::VecDeque::new(),
            clock: None,
        }
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn channels(&self) -> u16 {
        self.channels
    }

    // Blocks for the next raw chunk; None when the bridge closes (end of test).
    pub fn next_chunk(&self) -> Option<Vec<f32>> {
        self.rx.recv().ok()
    }

    // Clocks out the next 960-sample mono frame at an accurate 20 ms cadence.
    // Pulls PCM from the bridge receiver into an internal buffer, blocking until
    // a full frame's worth is available, then waits for the next 20 ms slot
    // before returning. None once the bridge closes and the buffer is drained.
    pub fn next_frame(&mut self) -> Option<Vec<f32>> {
        while self.pending.len() < FRAME_SAMPLES {
            match self.rx.recv() {
                Ok(chunk) => self.pending.extend(chunk),
                Err(_) => {
                    if self.pending.is_empty() {
                        return None;
                    }
                    break;
                }
            }
        }

        let take = FRAME_SAMPLES.min(self.pending.len());
        let mut frame: Vec<f32> = self.pending.drain(..take).collect();
        if frame.len() < FRAME_SAMPLES {
            frame.resize(FRAME_SAMPLES, 0.0);
        }

        // Start the clock on first frame so its zero point is the first emit.
        let clock = self.clock.get_or_insert_with(|| FrameClock::new(FRAME_INTERVAL_MS));
        clock.wait_next();

        Some(frame)
    }
}

#[cfg(all(test, feature = "e2e"))]
mod tests {
    use super::super::AudioFrame;
    use super::super::input::{MUTE_INPUT_STREAM, UPDATE_NOISE_GATE_SETTINGS, USE_NOISE_GATE};
    use super::super::input_core::InputProcessCore;
    use super::super::resampler::AudioResampler;
    use super::*;
    use audio_gate::NoiseGate;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    fn sine_440_48k_mono(frames: usize) -> Vec<f32> {
        let sample_rate = 48000.0_f32;
        let freq = 440.0_f32;
        (0..frames)
            .map(|n| (2.0 * std::f32::consts::PI * freq * n as f32 / sample_rate).sin() * 0.5)
            .collect()
    }

    // Mirrors InputStream::fake_listener: drive an InputProcessCore from a
    // BridgeInputSource feed loop and assert frames arrive on the producer.
    #[test]
    fn fake_source_feeds_frames_through_processing_core() {
        USE_NOISE_GATE.store(false, Ordering::Relaxed);
        UPDATE_NOISE_GATE_SETTINGS.store(false, Ordering::Relaxed);
        MUTE_INPUT_STREAM.store(false, Ordering::Relaxed);

        let (chunk_tx, chunk_rx) = flume::unbounded::<Vec<f32>>();
        let mut src = BridgeInputSource::new(chunk_rx, 48000, 1);

        let (producer, consumer) = flume::bounded::<AudioFrame>(10000);
        let shutdown = Arc::new(AtomicBool::new(false));

        let gate = NoiseGate::new(-36.0, -54.0, src.sample_rate() as f32, 1, 100.0, 1.0, 150.0);
        let audio_resampler = match AudioResampler::new_if_needed(src.sample_rate()) {
            Some(Ok(r)) => Some(r),
            _ => None,
        };

        let mut core = InputProcessCore::new(
            gate,
            audio_resampler,
            src.channels(),
            src.sample_rate(),
            2,
            producer,
        );

        let shutdown_thread = shutdown.clone();
        let handle = std::thread::spawn(move || {
            while !shutdown_thread.load(Ordering::Relaxed) {
                match src.next_frame() {
                    Some(frame) => core.process(&frame),
                    None => break,
                }
            }
        });

        // Send a 440 Hz sine through the bridge as 20 ms (960 sample) chunks
        let signal = sine_440_48k_mono(960 * 25);
        for chunk in signal.chunks(960) {
            chunk_tx.send(chunk.to_vec()).unwrap();
        }

        // Drop the sender so next_frame() returns None and the loop ends
        drop(chunk_tx);
        handle.join().unwrap();

        let frames: Vec<AudioFrame> = consumer.drain().collect();
        assert!(
            !frames.is_empty(),
            "expected frames from the fake feed loop, got {}",
            frames.len()
        );
    }
}
