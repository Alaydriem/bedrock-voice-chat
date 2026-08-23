use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// One participant's voice activity, as coarsely as a meter can be driven.
///
/// Deliberately not a level. A level has to be resent whenever it changes, and speech changes
/// at syllable rate — so carrying one costs a message several times a second per person, on a
/// transport where the message count is the whole cost. These two fields change on the order of
/// once a phrase, and the client generates the motion between them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ParticipantLevel {
    /// Whether audio is passing right now: past the gate, not muted, above the floor.
    pub speaking: bool,
    /// How loud, in `0..=LOUDNESS_STEPS` steps of the meter's own decibel scale.
    ///
    /// Quantised so the emitter has something that stops changing. A float changes on every
    /// frame and would put this back on the wire at capture rate.
    pub loudness: u8,
}

impl ParticipantLevel {
    /// Steps in the loudness scale, `0` being silent and this being full.
    ///
    /// Eight is the coarsest scale on which a meter still reads as continuous once the client
    /// eases between steps, and coarse is the point: every extra step is another value that can
    /// change, and every change is another message.
    pub const LOUDNESS_STEPS: u8 = 8;

    pub fn silent() -> Self {
        Self {
            speaking: false,
            loudness: 0,
        }
    }

    /// Whether this differs from `other` in any way a viewer could see.
    pub fn differs_from(&self, other: &Self) -> bool {
        self.speaking != other.speaking || self.loudness != other.loudness
    }
}
