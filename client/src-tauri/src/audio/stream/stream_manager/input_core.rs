use super::resampler::AudioResampler;
use super::{AudioFrame, AudioFrameData};
use audio_gate::NoiseGate;
use common::consts::OPUS_FRAME_DURATION_MS;
use common::structs::audio::{InputLevel, NoiseGateSettings};
use log::warn;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use super::input::{
    MUTE_INPUT_STREAM, NOISE_GATE_SETTINGS, UPDATE_NOISE_GATE_SETTINGS, USE_NOISE_GATE,
};
use crate::diagnostics::InputPipelineStats;

// Owned processing core for the input pipeline: noise gate, channel conversion,
// resampling, and silence/tail gating, terminating in an AudioFrame::F32 send.
pub(crate) struct InputProcessCore {
    gate: NoiseGate,
    audio_resampler: Option<AudioResampler>,
    src_channels: u16,
    #[allow(dead_code)]
    src_sample_rate: u32,
    pcm_buffer: Vec<f32>,
    consecutive_silent_frames: u32,
    tail_frame_count: u32,
    producer: flume::Sender<AudioFrame>,
    stats: Arc<InputPipelineStats>,
    // A channel rather than an AppHandle: an AppHandle-bearing field drags the
    // Tauri GUI into test binaries through drop glue. The owner emits.
    level_tx: flume::Sender<InputLevel>,
}

impl InputProcessCore {
    pub(crate) fn new(
        gate: NoiseGate,
        audio_resampler: Option<AudioResampler>,
        src_channels: u16,
        src_sample_rate: u32,
        tail_frame_count: u32,
        producer: flume::Sender<AudioFrame>,
        stats: Arc<InputPipelineStats>,
        level_tx: flume::Sender<InputLevel>,
    ) -> Self {
        Self {
            gate,
            audio_resampler,
            src_channels,
            src_sample_rate,
            pcm_buffer: vec![0.0; 4096],
            consecutive_silent_frames: 0,
            tail_frame_count,
            producer,
            stats,
            level_tx,
        }
    }

    pub(crate) fn process(&mut self, data: &[f32]) {
        let len = data.len();

        if self.pcm_buffer.len() < len {
            self.pcm_buffer.resize(len, 0.0);
        }

        self.pcm_buffer[..len].copy_from_slice(data);

        // If the noise gate is enabled, process data through it
        if USE_NOISE_GATE.load(Ordering::Relaxed) {
            // If there is a pending update, apply it, then disable the lock check
            if UPDATE_NOISE_GATE_SETTINGS.load(Ordering::Relaxed) {
                let current_settings = NOISE_GATE_SETTINGS.lock().unwrap();
                match serde_json::from_value::<NoiseGateSettings>(current_settings.clone()) {
                    Ok(settings) => {
                        log::info!("Updating noise gate settings: {:?}", settings);
                        self.gate.update(
                            settings.open_threshold,
                            settings.close_threshold,
                            settings.release_rate,
                            settings.attack_rate,
                            settings.hold_time,
                        );

                        self.tail_frame_count = (settings.release_rate
                            / OPUS_FRAME_DURATION_MS as f32)
                            .ceil()
                            .max(2.0) as u32;
                    }
                    Err(e) => {
                        warn!(
                            "Noise gate settings were asked to update, but failed to deserialize: {}",
                            e
                        );
                    }
                };
                drop(current_settings);

                UPDATE_NOISE_GATE_SETTINGS.store(false, Ordering::Relaxed);
            }

            // Process the frame in-place through the gate
            self.gate.process_frame(&mut self.pcm_buffer[..len]);
        }

        // Convert to mono if stereo
        let mono_pcm: Vec<f32> = if self.src_channels == 2 {
            self.pcm_buffer[..len]
                .chunks_exact(2)
                .map(|lr| (lr[0] + lr[1]) / 2.0)
                .collect()
        } else {
            self.pcm_buffer[..len].to_vec()
        };

        // Resample 44.1 kHz → 48 kHz if needed
        let mono_pcm = if let Some(ref mut rs) = self.audio_resampler {
            rs.process(&mono_pcm)
        } else {
            mono_pcm
        };

        // Silence and amplitude come out of one traversal. This is the capture
        // callback, so a second pass over the frame buys nothing.
        let mut sum_squares = 0.0f32;
        let mut nonzero = false;
        for &sample in mono_pcm.iter() {
            sum_squares += sample * sample;
            if sample != 0.0 {
                nonzero = true;
            }
        }
        let is_silent = !nonzero;
        let rms = if mono_pcm.is_empty() {
            0.0
        } else {
            (sum_squares / mono_pcm.len() as f32).sqrt()
        };

        let is_muted = MUTE_INPUT_STREAM.load(Ordering::Relaxed);

        // A frame that survives the gate with any nonzero sample is a frame the gate passed. The
        // gate exposes no open/closed query of its own, so this is the observation a diagnostic
        // reports as gate state.
        //
        // A hard mute counts as closed regardless of what the gate did: muting does not zero the
        // samples, so reporting the gate open while the mic is muted would print "gate open" and
        // "muted true" side by side in the same report.
        self.stats.record_frame(is_silent || is_muted);

        // Dropped rather than queued when the consumer is behind: a meter rendering
        // a level from two seconds ago is worse than one that skips a frame.
        let _ = self.level_tx.try_send(InputLevel {
            rms: if is_muted { 0.0 } else { rms },
            gate_open: !(is_silent || is_muted),
        });

        if is_muted {
            // Hard mute: reset state, send nothing
            self.consecutive_silent_frames = 0;
        } else if !is_silent {
            // Gate is open — real audio, reset counter, send frame
            self.consecutive_silent_frames = 0;

            let captured_at_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;

            match self.producer.try_send(AudioFrame::F32(AudioFrameData {
                pcm: mono_pcm,
                captured_at_ms,
            })) {
                Ok(()) => self.stats.record_sent(),
                Err(_e) => {}
            }
        } else if self.consecutive_silent_frames < self.tail_frame_count {
            // Gate just closed — send trailing silence frame
            self.consecutive_silent_frames += 1;

            let captured_at_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;

            match self.producer.try_send(AudioFrame::F32(AudioFrameData {
                pcm: mono_pcm,
                captured_at_ms,
            })) {
                Ok(()) => self.stats.record_sent(),
                Err(_e) => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_gate_disabled() -> NoiseGate {
        NoiseGate::new(-36.0, -54.0, 48000.0, 1, 100.0, 1.0, 150.0)
    }

    fn sine_440_48k_mono(frames: usize) -> Vec<f32> {
        let sample_rate = 48000.0_f32;
        let freq = 440.0_f32;
        (0..frames)
            .map(|n| (2.0 * std::f32::consts::PI * freq * n as f32 / sample_rate).sin() * 0.5)
            .collect()
    }

    fn run_once() -> Vec<AudioFrame> {
        // Ensure the noise gate is disabled for this deterministic DSP run
        USE_NOISE_GATE.store(false, Ordering::Relaxed);
        UPDATE_NOISE_GATE_SETTINGS.store(false, Ordering::Relaxed);
        MUTE_INPUT_STREAM.store(false, Ordering::Relaxed);

        let (producer, consumer) = flume::bounded::<AudioFrame>(10000);

        // 48 kHz mono → no resampler needed
        let (level_tx, _) = flume::unbounded();
        let mut core = InputProcessCore::new(
            make_gate_disabled(),
            None,
            1,
            48000,
            2,
            producer,
            Arc::new(InputPipelineStats::new()),
            level_tx,
        );

        // 20 ms at 48 kHz = 960 frames per chunk
        let chunk_frames = 960usize;
        let total_chunks = 50usize;
        let signal = sine_440_48k_mono(chunk_frames * total_chunks);

        for chunk in signal.chunks(chunk_frames) {
            core.process(chunk);
        }

        // Drop the core so the producer is dropped and the receiver drains cleanly
        drop(core);

        consumer.drain().collect()
    }

    fn encoded_bytes(frames: &[AudioFrame]) -> Vec<u8> {
        let mut out = Vec::new();
        for frame in frames {
            if let AudioFrame::F32(data) = frame {
                for sample in &data.pcm {
                    out.extend_from_slice(&sample.to_le_bytes());
                }
            }
        }
        out
    }

    #[test]
    fn input_process_core_produces_stable_deterministic_frames() {
        let first = run_once();
        let second = run_once();

        // A nonzero, stable number of frames is emitted from the 440 Hz tone
        assert!(
            !first.is_empty(),
            "expected nonzero AudioFrame output, got {}",
            first.len()
        );

        // Deterministic across two identical runs
        assert_eq!(
            first.len(),
            second.len(),
            "frame count differed across identical runs: {} vs {}",
            first.len(),
            second.len()
        );

        assert_eq!(
            encoded_bytes(&first),
            encoded_bytes(&second),
            "PCM payload differed across identical runs"
        );
    }
}
