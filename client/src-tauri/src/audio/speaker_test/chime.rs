use crate::audio::tone::{Tone, ToneSpec};

/// The two-note chime the speaker test plays, synthesised rather than shipped.
///
/// Two notes a fourth apart, each a small stack of partials under an exponential decay.
/// The partials are what stop it sounding like a test tone: a single sine reads as a fault
/// signal, and someone checking their speakers wants to recognise the sound as deliberate.
pub struct Chime;

impl Chime {
    const SPEC: ToneSpec = ToneSpec {
        // A5 then D6.
        notes: &[(880.0, 0.0), (1174.66, 0.17)],
        partials: &[(1.0, 1.0), (2.0, 0.32), (3.03, 0.11)],
        // Each note is most of the way down before the next lands.
        decay_seconds: 0.30,
        // A hard start on a sine is a click, which on a speaker test would be
        // indistinguishable from a fault in the very thing being tested.
        attack_seconds: 0.006,
        // Half scale: audible without being a shock on headphones, and with headroom so no
        // device clips a signal we generated ourselves.
        peak: 0.5,
        // Long enough to hear both notes ring out, short enough to press twice in a row.
        duration_seconds: 0.9,
    };

    /// Read by the speaker test, which holds its stream open for this long plus a tail.
    pub const DURATION_SECONDS: f32 = Chime::SPEC.duration_seconds;

    pub fn samples(sample_rate: u32, channels: u16) -> Vec<f32> {
        Tone::samples(&Chime::SPEC, sample_rate, channels)
    }
}
