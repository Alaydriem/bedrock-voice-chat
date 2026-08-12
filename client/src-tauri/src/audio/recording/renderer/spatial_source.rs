use common::structs::recording::RecordingHeader;

use crate::audio::recording::renderer::DecodedAudioFrame;
use crate::audio::spatial::{GainSmoother, SpatialGains, SpatialResolver};

/// One WAL key's frames, placed where the listener heard them.
///
/// Frames come in mono and leave interleaved stereo. The ramp is carried across the whole key
/// rather than reset per frame, and it is advanced across the gaps between frames as well:
/// playback's sink keeps pulling samples through a pause, so a voice that resumes after one
/// resumes where it left off.
pub struct SpatialSource;

impl SpatialSource {
    pub fn position(
        frames: Vec<(RecordingHeader, DecodedAudioFrame)>,
        resolver: &SpatialResolver,
    ) -> Vec<DecodedAudioFrame> {
        let mut smoother = GainSmoother::new(SpatialGains::centred());
        let mut target = SpatialGains::centred();
        let mut previous_end_ms: Option<u64> = None;
        let mut positioned = Vec::with_capacity(frames.len());

        for (header, frame) in frames {
            if let Some(resolved) = resolver.gains(&header) {
                target = resolved;
            }

            let mono = Self::to_mono(&frame.pcm_data, frame.channels);

            if let Some(end) = previous_end_ms {
                let gap_ms = frame.relative_timestamp_ms.saturating_sub(end);
                let gap_samples = (gap_ms as usize * frame.sample_rate as usize) / 1000;
                smoother.advance_by(&target, gap_samples);
            }

            let mut stereo = Vec::with_capacity(mono.len() * 2);
            for sample in &mono {
                let current = smoother.advance(&target);
                stereo.push(sample * current.volume * current.left);
                stereo.push(sample * current.volume * current.right);
            }

            let frame_ms = (mono.len() as u64 * 1000) / frame.sample_rate as u64;
            previous_end_ms = Some(frame.relative_timestamp_ms + frame_ms);

            positioned.push(DecodedAudioFrame {
                pcm_data: stereo,
                sample_rate: frame.sample_rate,
                channels: 2,
                relative_timestamp_ms: frame.relative_timestamp_ms,
            });
        }

        positioned
    }

    fn to_mono(samples: &[f32], channels: u16) -> Vec<f32> {
        if channels <= 1 {
            return samples.to_vec();
        }

        let channels = channels as usize;
        samples
            .chunks(channels)
            .map(|frame| frame.iter().sum::<f32>() / channels as f32)
            .collect()
    }
}
