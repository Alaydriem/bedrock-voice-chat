use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{ActiveConnection, ConnectTarget, VoiceMode};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SuccessResponse {
    pub success: bool,
    pub data: ResponseData,
}

/// An untagged enum is tried in declaration order, so a new variant goes at the end: a
/// permissive shape placed earlier would swallow payloads meant for a later one.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ResponseData {
    Pong(PongData),
    Mute(MuteData),
    Record(RecordData),
    State(StateData),
    Ptt(PttData),
    Targets(TargetsData),
    Connect(ConnectData),
    Jukebox(JukeboxData),
    Group(GroupData),
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
pub struct JukeboxData {
    pub muted: bool,
    /// The level as a fraction, where 1.0 is untouched.
    ///
    /// Reported alongside `muted` so a controller learns both halves from either command and never
    /// has to infer one from the other — they are independent, and a level survives a mute.
    pub gain: f32,
}

/// Whether this client is in a group, and which.
///
/// `id` and `name` are absent when a leave found nothing to leave, matching how `ConnectData`
/// reports a disconnect that found nothing running.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GroupData {
    pub in_group: bool,
    pub id: Option<String>,
    pub name: Option<String>,
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
    /// Whether jukebox music is muted.
    pub jukebox_muted: bool,
    /// How loud jukebox music plays, as a fraction where 1.0 is untouched.
    pub jukebox_gain: f32,
    /// The world this client is connected to, absent when nothing is running.
    pub connection: Option<ActiveConnection>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TargetsData {
    pub targets: Vec<ConnectTarget>,
}

/// The outcome of a connect or a disconnect, so a caller can fail fast rather than poll for
/// a state change that may never come.
///
/// `id` and `name` are absent when a disconnect found nothing running.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ConnectData {
    pub connected: bool,
    pub id: Option<String>,
    pub name: Option<String>,
}

impl SuccessResponse {
    pub fn targets(targets: Vec<ConnectTarget>) -> Self {
        Self {
            success: true,
            data: ResponseData::Targets(TargetsData { targets }),
        }
    }

    pub fn connect(id: String, name: String) -> Self {
        Self {
            success: true,
            data: ResponseData::Connect(ConnectData {
                connected: true,
                id: Some(id),
                name: Some(name),
            }),
        }
    }

    pub fn disconnect(id: Option<String>, name: Option<String>) -> Self {
        Self {
            success: true,
            data: ResponseData::Connect(ConnectData {
                connected: false,
                id,
                name,
            }),
        }
    }

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

    pub fn jukebox(muted: bool, gain: f32) -> Self {
        Self {
            success: true,
            data: ResponseData::Jukebox(JukeboxData { muted, gain }),
        }
    }
}
