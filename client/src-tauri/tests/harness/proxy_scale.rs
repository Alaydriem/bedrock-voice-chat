// Shared test-actor scales + Goertzel assertion helpers; not every scenario file
// references every actor or helper.
#![allow(dead_code)]

use bvc_client_lib::testkit::signal::Signal;

/// A test actor's distinct triad (root/third/fifth). Scales are chosen so that
/// each pair used as cross-measurement subjects in a single test is disjoint
/// after the Opus round-trip. Alice (C4) and Carol (F5) are harmonically
/// adjacent and must NOT be the two speakers in a `silent_of` assertion pair;
/// all other pairings are clean.
#[derive(Clone, Copy)]
pub struct Scale {
    pub name: &'static str,
    pub freqs: [f32; 3],
}

// C4 major triad: C4 E4 G4
pub const ALICE: Scale = Scale { name: "Alice", freqs: [261.63, 329.63, 392.00] };
// A4 major triad: A4 C#5 E5
pub const BOB: Scale = Scale { name: "Bob", freqs: [440.00, 554.37, 659.25] };
// F5 major triad: F5 A5 C6
pub const CAROL: Scale = Scale { name: "Carol", freqs: [698.46, 880.00, 1046.50] };
// D6 major triad: D6 F#6 A6
pub const DAVE: Scale = Scale { name: "Dave", freqs: [1174.66, 1479.98, 1760.00] };

impl Scale {
    /// "1 3 5 3 1" melody then the triad chord, repeated `repeats` times.
    pub fn voice(&self, repeats: u32) -> Vec<f32> {
        let [d1, d3, d5] = self.freqs;
        Signal::progression(&[d1, d3, d5, d3, d1], &[d1, d3, d5], 0.4, 0.6, repeats, 48_000)
    }
}

/// True when every note of `scale` carries > 2% Goertzel energy in the mono capture.
pub fn hears(mono: &[f32], scale: Scale) -> bool {
    scale.freqs.iter().all(|&f| Signal::tone_energy_fraction(mono, 48_000, f) > 0.02)
}

/// True when NO note of `scale` carries > 1% energy (absent / cross-bleed-free).
pub fn silent_of(mono: &[f32], scale: Scale) -> bool {
    scale.freqs.iter().all(|&f| Signal::tone_energy_fraction(mono, 48_000, f) < 0.01)
}
