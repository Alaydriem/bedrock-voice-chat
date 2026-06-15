use std::time::Instant;

pub struct PendingInject {
    pub token: String,
    pub deadline: Instant,
}

impl PendingInject {
    pub fn is_expired(&self, now: Instant) -> bool {
        now >= self.deadline
    }
}
