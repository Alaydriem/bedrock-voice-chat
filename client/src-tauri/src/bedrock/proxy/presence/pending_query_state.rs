use std::time::Instant;

/// One encoded `!bvcs:` reverse-ride message awaiting injection into the
/// proxied session as serverbound chat. Carries a deadline so state that sat in
/// the queue (no session, stalled loop) is dropped rather than injected stale.
pub struct PendingQueryState {
    pub message: String,
    pub deadline: Instant,
}

impl PendingQueryState {
    pub fn is_expired(&self, now: Instant) -> bool {
        now >= self.deadline
    }
}
