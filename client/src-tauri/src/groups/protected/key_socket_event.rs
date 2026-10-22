use common::structs::group_key::GroupKeyFrame;

/// What the key socket hands the group's runner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeySocketEvent {
    Frame(GroupKeyFrame),
    /// The socket ended. The reason is the server's close reason, when it gave one.
    Closed(Option<String>),
}
