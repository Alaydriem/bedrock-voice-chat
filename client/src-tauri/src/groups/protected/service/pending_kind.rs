use common::structs::group_key::SocketId;

/// What an unacknowledged commit does, so the service knows how to finish or retry it.
#[derive(Debug, Clone)]
pub enum PendingKind {
    /// Adds a joiner; the Welcome goes to its socket once the relay accepts the commit.
    Add {
        joiner: SocketId,
        key_package: Vec<u8>,
        welcome: Vec<u8>,
    },
    Remove,
    Update,
}
