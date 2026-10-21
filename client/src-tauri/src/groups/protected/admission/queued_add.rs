use std::time::{Duration, Instant};

use common::structs::group_key::SocketId;

/// An admitted joiner waiting for its Add commit: exactly the KeyPackage its check stored, tied
/// to that check's id until the Welcome goes out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueuedAdd {
    pub check_id: [u8; 16],
    pub joiner: SocketId,
    pub key_package: Vec<u8>,
    /// When the member sent its `Check`.
    pub checked_at: Instant,
}

impl QueuedAdd {
    /// The joiner stops waiting for its Welcome 120 s after it asked. An Add started later than
    /// this after the check would add a leaf nobody holds the keys for.
    pub const MAX_AGE: Duration = Duration::from_secs(90);

    pub fn is_too_old(&self, now: Instant) -> bool {
        now.duration_since(self.checked_at) >= Self::MAX_AGE
    }
}
