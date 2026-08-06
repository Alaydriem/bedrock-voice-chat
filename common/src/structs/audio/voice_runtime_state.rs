use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::structs::keybinds::VoiceMode;

/// What the audio backend believes about this microphone, right now.
///
/// Reported rather than inferred. Mute lives in a process-global flag on the capture
/// stream, the voice mode lives on the keybind listener, and the UI keeps its own copy of
/// both — so "the button says unmuted" is evidence about the third copy and nothing else.
/// In push-to-talk the button deliberately never draws the muted glyph, which leaves no
/// surface at all showing what the microphone is actually doing.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct VoiceRuntimeState {
    pub voice_mode: VoiceMode,
    /// Whether a push-to-talk hold is registered. Always false in open mic.
    pub ptt_active: bool,
    /// The flag the capture stream reads. `true` means frames are zeroed at the source.
    pub input_muted: bool,
    pub output_muted: bool,
}
