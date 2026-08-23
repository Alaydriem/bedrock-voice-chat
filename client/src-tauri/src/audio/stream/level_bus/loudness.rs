use common::structs::audio::ParticipantLevel;

/// A measured RMS turned into a loudness step, with enough hysteresis to stop changing.
///
/// The scale is the meter's own: decibels across a window chosen to be generous rather than
/// accurate, matching `LevelScale` on the client so a step here means the same height there.
/// RMS is linear in pressure and hearing is not, so a linear split would spend seven of eight
/// steps on levels nobody produces.
///
/// The hysteresis is what makes this worth quantising at all. Without it a voice sitting on a
/// boundary flips between two steps every frame, and since a step change is what puts a message
/// on the wire, a steady voice would cost as much as a changing one.
pub struct LoudnessTracker {
    current: u8,
}

impl LoudnessTracker {
    /// Below this reads as silence. Roughly a quiet room once the gate has had its say.
    const FLOOR_DB: f32 = -55.0;

    /// At or above this the meter is full. Well under clipping, deliberately.
    const CEILING_DB: f32 = -18.0;

    /// How far past a boundary a level must go before the step follows it, as a fraction of one
    /// step. A voice wandering either side of a boundary holds its step instead of oscillating.
    const HYSTERESIS: f32 = 0.35;

    pub fn new() -> Self {
        Self { current: 0 }
    }

    /// Fold one measurement in and report the participant state to publish.
    ///
    /// `passing` is whether the audio path let this frame through at all — past the gate and
    /// not muted. It is a veto, not the answer: a loud frame the gate discarded is not somebody
    /// speaking, but a frame that survived is not automatically somebody speaking either.
    ///
    /// The floor decides the rest, and it has to. `passing` was the whole answer once, and with
    /// the noise gate switched off it is derived from whether any sample is exactly zero — which
    /// a live microphone never satisfies. So a speaker read as speaking forever, never
    /// transitioned, and their own meter went still while everyone else's moved: a state that
    /// never changes is never worth a message, so the only thing left driving it was the
    /// keepalive.
    ///
    /// Nothing is lost when the gate *is* on: its own threshold is well above this floor, so
    /// audio it opened for always lands above step zero.
    pub fn observe(&mut self, rms: f32, passing: bool) -> ParticipantLevel {
        if !passing {
            self.current = 0;
            return ParticipantLevel::silent();
        }

        let steps = ParticipantLevel::LOUDNESS_STEPS;
        let exact = Self::exact_step(rms, steps);

        // Moved only once the reading is further from the current step than half a step plus
        // the margin, so a level resting on a boundary stays put instead of oscillating.
        if (exact - self.current as f32).abs() >= 0.5 + Self::HYSTERESIS {
            self.current = exact.round().clamp(0.0, steps as f32) as u8;
        }

        ParticipantLevel {
            speaking: self.current > 0,
            loudness: self.current,
        }
    }

    fn exact_step(rms: f32, steps: u8) -> f32 {
        // Guards NaN as well as zero and negatives: log10(0) is negative infinity, and a gated
        // frame arrives as exactly 0.
        if !(rms > 0.0) {
            return 0.0;
        }

        let db = 20.0 * rms.log10();
        let span = Self::CEILING_DB - Self::FLOOR_DB;
        let fraction = ((db - Self::FLOOR_DB) / span).clamp(0.0, 1.0);
        fraction * steps as f32
    }
}

impl Default for LoudnessTracker {
    fn default() -> Self {
        Self::new()
    }
}
