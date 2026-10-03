use serde::{Deserialize, Serialize};

/// The action a `ClientAction` carries.
///
/// Externally tagged: a variant with no data is the bare string (`"CreateGroup"`), and a
/// variant with data is an object keyed by the variant name (`{"SetMuted": true}`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub enum ClientActionType {
    /// Mutes (`true`) or unmutes (`false`) the actor's microphone.
    #[cfg_attr(feature = "openapi", schemars(title = "SetMuted"))]
    SetMuted(bool),
    /// Deafens (`true`) or undeafens (`false`) the actor.
    #[cfg_attr(feature = "openapi", schemars(title = "SetDeafened"))]
    SetDeafened(bool),
    /// Starts (`true`) or stops (`false`) a recording on the actor's client. Starting one
    /// returns 403 when the server does not permit recording.
    #[cfg_attr(feature = "openapi", schemars(title = "SetRecording"))]
    SetRecording(bool),
    /// Sets the playback gain the actor hears `target` at. `volume` is a linear gain,
    /// clamped to 0.0–1.5; a non-finite value is ignored.
    #[cfg_attr(feature = "openapi", schemars(title = "SetVolume"))]
    SetVolume { target: String, volume: f32 },
    /// Mutes (`muted: true`) or unmutes `target` for the actor only.
    #[cfg_attr(feature = "openapi", schemars(title = "SetHeard"))]
    SetHeard { target: String, muted: bool },
    /// Creates a group, moves the actor into it and returns the new group's ID.
    #[cfg_attr(feature = "openapi", schemars(title = "CreateGroup"))]
    CreateGroup,
    /// Moves the actor into the group with the share code `channel`. Case, spacing and the
    /// dash are ignored. An unknown code returns 404 and leaves the actor where they are.
    #[cfg_attr(feature = "openapi", schemars(title = "JoinGroup"))]
    JoinGroup { channel: String },
    /// Removes the actor from their group.
    #[cfg_attr(feature = "openapi", schemars(title = "LeaveGroup"))]
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
