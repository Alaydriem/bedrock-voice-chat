use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::VoiceMode;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SuccessResponse {
    pub success: bool,
    pub data: ResponseData,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ResponseData {
    Pong(PongData),
    Mute(MuteData),
    Record(RecordData),
    State(StateData),
    Ptt(PttData),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PongData {
    pub pong: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MuteData {
    pub device: String,
    pub muted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RecordData {
    pub recording: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PttData {
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StateData {
    pub muted: bool,
    pub deafened: bool,
    pub recording: bool,
    /// What the mute control means. A controller that ignores this offers a toggle for a
    /// state push-to-talk already owns.
    pub voice_mode: VoiceMode,
    /// Whether push-to-talk is held right now. Always false in `openMic`.
    pub ptt_active: bool,
}

impl SuccessResponse {
    pub fn pong() -> Self {
        Self {
            success: true,
            data: ResponseData::Pong(PongData { pong: true }),
        }
    }

    pub fn mute(device: String, muted: bool) -> Self {
        Self {
            success: true,
            data: ResponseData::Mute(MuteData { device, muted }),
        }
    }

    pub fn record(recording: bool) -> Self {
        Self {
            success: true,
            data: ResponseData::Record(RecordData { recording }),
        }
    }

    pub fn state(state: StateData) -> Self {
        Self {
            success: true,
            data: ResponseData::State(state),
        }
    }

    pub fn ptt(active: bool) -> Self {
        Self {
            success: true,
            data: ResponseData::Ptt(PttData { active }),
        }
    }
}
