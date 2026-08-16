use std::time::Duration;

// How long to wait before dialling a peer again.
//
// Holds a count rather than reading a clock, so the schedule is decided here and
// the waiting happens in the caller. That is what makes it testable without
// sleeping.
pub struct Backoff {
    attempt: u32,
}

impl Backoff {
    // A dropped link is most often a peer restarting, which is over in seconds.
    pub const FIRST: Duration = Duration::from_millis(500);

    // Past this, a longer wait buys nothing: the peer is down rather than busy,
    // and one dial a half-minute costs nothing to keep making.
    pub const CEILING: Duration = Duration::from_secs(30);

    pub fn new() -> Self {
        Self { attempt: 0 }
    }

    pub fn next_delay(&mut self) -> Duration {
        let multiplier = 1u32.checked_shl(self.attempt).unwrap_or(u32::MAX);
        let delay = Self::FIRST
            .checked_mul(multiplier)
            .unwrap_or(Self::CEILING)
            .min(Self::CEILING);

        self.attempt = self.attempt.saturating_add(1);
        delay
    }

    pub fn reset(&mut self) {
        self.attempt = 0;
    }
}

impl Default for Backoff {
    fn default() -> Self {
        Self::new()
    }
}
