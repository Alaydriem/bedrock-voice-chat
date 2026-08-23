mod spec;

pub use spec::ToneSpec;

/// Renders a `ToneSpec` to interleaved samples at a requested rate.
///
/// Generated rather than shipped because a file would be a second thing to keep in sync
/// with the device's sample rate. A tone built at whatever rate the endpoint reports needs
/// no resampling and cannot be the wrong format for the device playing it.
pub struct Tone;

impl Tone {
    /// Interleaved samples for the given rate and channel count.
    ///
    /// Every channel gets the same signal. A sound that only came out of one side would
    /// have someone chasing a fault that is in the tone.
    pub fn samples(spec: &ToneSpec, sample_rate: u32, channels: u16) -> Vec<f32> {
        let mono = Tone::mono(spec, sample_rate);
        let channels = channels.max(1) as usize;

        let mut interleaved = Vec::with_capacity(mono.len() * channels);
        for sample in mono {
            for _ in 0..channels {
                interleaved.push(sample);
            }
        }
        interleaved
    }

    fn mono(spec: &ToneSpec, sample_rate: u32) -> Vec<f32> {
        let rate = sample_rate.max(1) as f32;
        let frames = (rate * spec.duration_seconds).ceil() as usize;

        let mut samples = Vec::with_capacity(frames);
        let mut peak = 0.0f32;

        for frame in 0..frames {
            let t = frame as f32 / rate;
            let mut value = 0.0f32;

            for (frequency, start) in spec.notes {
                let age = t - start;
                if age < 0.0 {
                    continue;
                }

                let envelope =
                    (-age / spec.decay_seconds).exp() * (age / spec.attack_seconds).min(1.0);

                for (multiple, amplitude) in spec.partials {
                    let phase = std::f32::consts::TAU * frequency * multiple * age;
                    value += phase.sin() * amplitude * envelope;
                }
            }

            peak = peak.max(value.abs());
            samples.push(value);
        }

        // Normalised from the measured peak rather than a computed one: the partials sum
        // constructively at the attack and the notes overlap, so the true maximum is not
        // something worth deriving by hand every time a partial changes.
        if peak > 0.0 {
            let scale = spec.peak / peak;
            for sample in samples.iter_mut() {
                *sample *= scale;
            }
        }

        samples
    }
}
