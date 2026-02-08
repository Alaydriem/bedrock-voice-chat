use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// Success response wrapper
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SuccessResponse {
    pub success: bool,
    pub data: ResponseData,
}

/// Response data variants
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ResponseData {
    /// Ping response
    Pong(PongData),
    /// Mute response with status
    Mute(MuteData),
    /// Recording response with status
    Record(RecordData),
    /// Full state response
    State(StateData),
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
pub struct StateData {
    pub muted: bool,
    pub deafened: bool,
    pub recording: bool,
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

    pub fn state(muted: bool, deafened: bool, recording: bool) -> Self {
        Self {
            success: true,
            data: ResponseData::State(StateData { muted, deafened, recording }),
        }
    }
}
