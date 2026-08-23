use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// What the noise gate is doing to this microphone, right now.
///
/// Three states rather than a boolean, because a boolean could not distinguish the two
/// that matter most to someone whose microphone has gone quiet: a gate that is switched
/// off passes everything, and a gate that is open passes everything, so "is audio getting
/// through" reads identically for both. A reader could not tell whether the gate was even
/// attached, and the natural inference from a row saying `open` was that it was — which is
/// exactly the wrong place to go looking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum NoiseGateStatus {
    /// Not bound to the audio path. Every captured frame reaches the encoder untouched.
    Disabled,
    /// Bound, and passing audio.
    Open,
    /// Bound, and cutting. The one state that can be the reason a microphone is silent.
    Closed,
}

impl NoiseGateStatus {
    /// `enabled` is the flag the capture path itself reads. `passing` is whether any frame
    /// carried signal over the sampling interval — which says something about the gate
    /// only once the gate is actually in the path.
    pub fn of(enabled: bool, passing: bool) -> Self {
        match (enabled, passing) {
            (false, _) => Self::Disabled,
            (true, true) => Self::Open,
            (true, false) => Self::Closed,
        }
    }

    /// Whether the gate is in the audio path at all.
    pub fn is_enabled(&self) -> bool {
        !matches!(self, Self::Disabled)
    }

    /// Whether the gate is the reason nothing is getting through.
    pub fn is_cutting(&self) -> bool {
        matches!(self, Self::Closed)
    }
}
