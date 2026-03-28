use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::voice_mode::VoiceMode;

#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[serde(default, rename_all = "camelCase")]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct KeybindConfig {
    pub toggle_mute: String,
    pub toggle_deafen: String,
    pub toggle_recording: String,
    pub push_to_talk: String,
    pub voice_mode: VoiceMode,
}

impl Default for KeybindConfig {
    fn default() -> Self {
        Self {
            toggle_mute: "ControlLeft+BracketLeft".to_string(),
            toggle_deafen: "ControlLeft+BracketRight".to_string(),
            toggle_recording: "ControlLeft+Backslash".to_string(),
            push_to_talk: "Backquote".to_string(),
            voice_mode: VoiceMode::default(),
        }
    }
}
