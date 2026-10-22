use super::PendingKind;

/// A commit sent to the relay whose echo, or a rival commit for its epoch, has not arrived yet.
pub struct PendingCommit {
    pub kind: PendingKind,
    pub bytes: Vec<u8>,
    /// The epoch it builds on.
    pub epoch: u64,
}
