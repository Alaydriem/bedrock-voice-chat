/// The two-note chime the speaker test plays, synthesised rather than shipped.
///
/// Generated because a file would be a second thing to keep in sync with the device's
/// sample rate. A tone built at whatever rate the endpoint reports needs no resampling and
/// cannot be the wrong format for the device it is meant to be testing.
///
/// Two notes a fourth apart, each a small stack of partials under an exponential decay.
/// The partials are what stop it sounding like a test tone: a single sine reads as a fault
/// signal, and someone checking their speakers wants to recognise the sound as deliberate.
pub struct Chime;

impl Chime {
    /// Long enough to hear both notes ring out, short enough to press twice in a row.
    pub const DURATION_SECONDS: f32 = 0.9;

    /// (frequency in Hz, start time in seconds). A5 then D6.
    const NOTES: [(f32, f32); 2] = [(880.0, 0.0), (1174.66, 0.17)];

    /// (harmonic multiple, relative amplitude). Falling amplitude with height, as a struck
    /// object behaves; equal partials sound like a buzzer.
    const PARTIALS: [(f32, f32); 3] = [(1.0, 1.0), (2.0, 0.32), (3.03, 0.11)];

    /// Exponential decay constant. Each note is most of the way down before the next lands.
    const DECAY_SECONDS: f32 = 0.30;

    /// A hard start on a sine is a click, which on a speaker test would be indistinguishable
    /// from a fault in the very thing being tested.
    const ATTACK_SECONDS: f32 = 0.006;

    /// Peak after normalisation. Half scale: audible without being a shock on headphones,
    /// and with headroom so no device clips a signal we generated ourselves.
    const PEAK: f32 = 0.5;

    /// Interleaved samples for the given rate and channel count.
    ///
    /// Every channel gets the same signal. A test that only came out of one side would
    /// have someone chasing a fault that is in the tone.
    pub fn samples(sample_rate: u32, channels: u16) -> Vec<f32> {
        let mono = Chime::mono(sample_rate);
        let channels = channels.max(1) as usize;

        let mut interleaved = Vec::with_capacity(mono.len() * channels);
        for sample in mono {
            for _ in 0..channels {
                interleaved.push(sample);
            }
        }
        interleaved
    }

    fn mono(sample_rate: u32) -> Vec<f32> {
        let rate = sample_rate.max(1) as f32;
        let frames = (rate * Chime::DURATION_SECONDS).ceil() as usize;

        let mut samples = Vec::with_capacity(frames);
        let mut peak = 0.0f32;

        for frame in 0..frames {
            let t = frame as f32 / rate;
            let mut value = 0.0f32;

            for (frequency, start) in Chime::NOTES {
                let age = t - start;
                if age < 0.0 {
                    continue;
                }

                let envelope = (-age / Chime::DECAY_SECONDS).exp()
                    * (age / Chime::ATTACK_SECONDS).min(1.0);

                for (multiple, amplitude) in Chime::PARTIALS {
                    let phase = std::f32::consts::TAU * frequency * multiple * age;
                    value += phase.sin() * amplitude * envelope;
                }
            }

            peak = peak.max(value.abs());
            samples.push(value);
        }

        // Normalised from the measured peak rather than a computed one: the partials sum
        // constructively at the attack and the two notes overlap, so the true maximum is
        // not something worth deriving by hand every time a partial changes.
        if peak > 0.0 {
            let scale = Chime::PEAK / peak;
            for sample in samples.iter_mut() {
                *sample *= scale;
            }
        }

        samples
    }
}
