use crate::audio::recording::renderer::DecodedAudioFrame;

/// Several sources onto one timeline.
///
/// Frames carry their offset from the start of the session, so laying them down is a
/// matter of position rather than order, and a gap between two of them is silence that
/// has to survive into the output.
pub struct TrackMixer;

impl TrackMixer {
    pub fn mix(sources: &[Vec<DecodedAudioFrame>], sample_rate: u32, channels: u16) -> Vec<f32> {
        let per_ms = (sample_rate as usize * channels as usize) / 1000;

        let length = sources
            .iter()
            .flat_map(|frames| frames.iter())
            .map(|frame| Self::offset(frame, per_ms) + frame.pcm_data.len())
            .max()
            .unwrap_or(0);

        let mut timeline = vec![0.0f32; length];
        for frame in sources.iter().flat_map(|frames| frames.iter()) {
            let at = Self::offset(frame, per_ms);
            for (index, sample) in frame.pcm_data.iter().enumerate() {
                timeline[at + index] += sample;
            }
        }

        // Summed sources run past full scale, and everything downstream expects a signal
        // inside it.
        for sample in timeline.iter_mut() {
            *sample = sample.clamp(-1.0, 1.0);
        }

        timeline
    }

    fn offset(frame: &DecodedAudioFrame, per_ms: usize) -> usize {
        frame.relative_timestamp_ms as usize * per_ms
    }

    /// When the earliest of these sources starts, in milliseconds from the session's own
    /// zero.
    ///
    /// A track that begins forty minutes in must not carry forty minutes of encoded
    /// silence to say so. Every other track states its start as a timecode and begins its
    /// samples at its first packet; this is the number that lets a mixed one do the same.
    pub fn lead_ms(sources: &[Vec<DecodedAudioFrame>]) -> u64 {
        sources
            .iter()
            .flat_map(|frames| frames.iter())
            .map(|frame| frame.relative_timestamp_ms)
            .min()
            .unwrap_or(0)
    }

    /// The mix with that lead removed, so sample zero is the first audio there is.
    pub fn mix_from_first_sound(
        sources: &[Vec<DecodedAudioFrame>],
        sample_rate: u32,
        channels: u16,
    ) -> (Vec<f32>, u64) {
        let lead = Self::lead_ms(sources);
        let mixed = Self::mix(sources, sample_rate, channels);
        let per_ms = (sample_rate as usize * channels as usize) / 1000;
        let cut = (lead as usize * per_ms).min(mixed.len());

        (mixed[cut..].to_vec(), lead)
    }
}
