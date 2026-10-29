use common::structs::relay::wire::datagram::GroupScope;

// The linked group a frame belongs to, flattened for FFI.
//
// `channel` is the BVC channel id. `members_only` speech is for the group's members alone: the
// bridge plays it to them and to nobody nearby.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct SdkGroupScope {
    pub channel: String,
    pub members_only: bool,
}

impl From<GroupScope> for SdkGroupScope {
    fn from(group: GroupScope) -> Self {
        Self {
            channel: group.channel,
            members_only: group.members_only,
        }
    }
}

impl From<SdkGroupScope> for GroupScope {
    fn from(group: SdkGroupScope) -> Self {
        Self {
            channel: group.channel,
            members_only: group.members_only,
        }
    }
}
