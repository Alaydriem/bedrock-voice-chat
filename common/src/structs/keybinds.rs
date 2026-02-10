use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum VoiceMode {
    #[default]
    OpenMic,
    PushToTalk,
}

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
            toggle_mute: "BracketLeft".to_string(),
            toggle_deafen: "BracketRight".to_string(),
            toggle_recording: "Backslash".to_string(),
            push_to_talk: "Backquote".to_string(),
            voice_mode: VoiceMode::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum KeybindAction {
    ToggleMute,
    ToggleDeafen,
    ToggleRecording,
    PushToTalk,
}

/// PTT state event emitted to the frontend via Tauri events.
/// The Display/to_string() of the variant is the event name.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum PttEvent {
    #[serde(rename = "ptt:active")]
    Active,
}

impl std::fmt::Display for PttEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PttEvent::Active => write!(f, "ptt:active"),
        }
    }
}
