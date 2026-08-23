mod policy;
mod sink;

pub use policy::CuePolicy;
pub use sink::CueSink;

use crate::audio::tone::{Tone, ToneSpec};

/// A short tone that reports a change the user cannot see from inside the game.
///
/// Descending for off, ascending for on, which is the convention every other voice client
/// uses and therefore the one that needs no explaining. Mute is two notes and deafen is
/// three, so the two states are distinguishable by ear rather than by counting semitones.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cue {
    Mute,
    Unmute,
    Deafen,
    Undeafen,
}

impl Cue {
    /// Softer and shorter than the speaker-test chime, which fires once during setup where
    /// this fires dozens of times a session.
    const PARTIALS: [(f32, f32); 2] = [(1.0, 1.0), (2.0, 0.25)];
    const DECAY_SECONDS: f32 = 0.05;
    const ATTACK_SECONDS: f32 = 0.004;
    const PEAK: f32 = 0.25;

    const MUTE: ToneSpec = ToneSpec {
        notes: &[(660.0, 0.0), (440.0, 0.055)],
        partials: &Cue::PARTIALS,
        decay_seconds: Cue::DECAY_SECONDS,
        attack_seconds: Cue::ATTACK_SECONDS,
        peak: Cue::PEAK,
        duration_seconds: 0.20,
    };

    const UNMUTE: ToneSpec = ToneSpec {
        notes: &[(440.0, 0.0), (660.0, 0.055)],
        partials: &Cue::PARTIALS,
        decay_seconds: Cue::DECAY_SECONDS,
        attack_seconds: Cue::ATTACK_SECONDS,
        peak: Cue::PEAK,
        duration_seconds: 0.20,
    };

    const DEAFEN: ToneSpec = ToneSpec {
        notes: &[(440.0, 0.0), (330.0, 0.05), (220.0, 0.10)],
        partials: &Cue::PARTIALS,
        decay_seconds: Cue::DECAY_SECONDS,
        attack_seconds: Cue::ATTACK_SECONDS,
        peak: Cue::PEAK,
        duration_seconds: 0.26,
    };

    const UNDEAFEN: ToneSpec = ToneSpec {
        notes: &[(220.0, 0.0), (330.0, 0.05), (440.0, 0.10)],
        partials: &Cue::PARTIALS,
        decay_seconds: Cue::DECAY_SECONDS,
        attack_seconds: Cue::ATTACK_SECONDS,
        peak: Cue::PEAK,
        duration_seconds: 0.26,
    };

    fn spec(&self) -> ToneSpec {
        match self {
            Cue::Mute => Cue::MUTE,
            Cue::Unmute => Cue::UNMUTE,
            Cue::Deafen => Cue::DEAFEN,
            Cue::Undeafen => Cue::UNDEAFEN,
        }
    }

    pub fn duration_seconds(&self) -> f32 {
        self.spec().duration_seconds
    }

    pub fn samples(&self, sample_rate: u32, channels: u16) -> Vec<f32> {
        Tone::samples(&self.spec(), sample_rate, channels)
    }
}
