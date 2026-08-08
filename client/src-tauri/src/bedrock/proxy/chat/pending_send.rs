use std::time::Instant;

/// One app-composed line awaiting injection into the proxied session as serverbound chat.
///
/// Carries a deadline for the same reason the state rides do: a message that sat in the queue
/// because no session was running is better dropped than delivered minutes late into a
/// conversation that has moved on.
pub struct PendingChatSend {
    pub text: String,
    pub deadline: Instant,
}

impl PendingChatSend {
    pub fn is_expired(&self, now: Instant) -> bool {
        now >= self.deadline
    }
}
