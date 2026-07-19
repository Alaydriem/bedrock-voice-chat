use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub enum ClientActionType {
    SetMuted(bool),
    SetDeafened(bool),
    SetRecording(bool),
    SetVolume { target: String, volume: f32 },
    SetHeard { target: String, muted: bool },
    CreateGroup,
    JoinGroup { channel: String },
    LeaveGroup,
}

impl ClientActionType {
    pub fn is_group_action(&self) -> bool {
        matches!(
            self,
            ClientActionType::CreateGroup
                | ClientActionType::JoinGroup { .. }
                | ClientActionType::LeaveGroup
        )
    }
}
