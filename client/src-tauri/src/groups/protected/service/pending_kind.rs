use crate::groups::protected::QueuedAdd;

/// What an unacknowledged commit does, so the service knows how to finish or retry it.
#[derive(Debug, Clone)]
pub enum PendingKind {
    /// Adds an admitted joiner; the Welcome goes to its socket, under its check's id, once the
    /// relay accepts the commit.
    Add { add: QueuedAdd, welcome: Vec<u8> },
    Remove,
    Update,
}
