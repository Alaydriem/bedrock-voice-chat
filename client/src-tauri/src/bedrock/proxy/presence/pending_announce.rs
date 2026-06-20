use std::time::Instant;

pub struct PendingAnnounce {
    pub endpoint: String,
    pub deadline: Instant,
}

impl PendingAnnounce {
    pub fn is_expired(&self, now: Instant) -> bool {
        now >= self.deadline
    }
}
