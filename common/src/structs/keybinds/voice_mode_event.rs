use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Emitted when the voice mode changes, whoever changed it.
///
/// Settings, the Stream Deck and a future in-game command all write the same setting, and
/// the mic button is a hold control in one mode and a toggle in the other. Without this the
/// dashboard reads the mode once at start-up and keeps offering the wrong control.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum VoiceModeEvent {
    #[serde(rename = "voice-mode:changed")]
    Changed,
}

impl std::fmt::Display for VoiceModeEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VoiceModeEvent::Changed => write!(f, "voice-mode:changed"),
        }
    }
}
