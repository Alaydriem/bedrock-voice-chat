use std::time::Duration;

/// How long to wait before the next try of something that keeps failing.
///
/// Doubles from `initial` and stops growing at `cap`. A registry that is down, or that has
/// refused a request for a reason that lasts, is asked again at the cap rather than at a
/// rate that makes the refusal worse.
pub struct PlayBackoff {
    next: Duration,
    cap: Duration,
}

impl PlayBackoff {
    pub fn new(initial: Duration, cap: Duration) -> Self {
        Self {
            next: initial.min(cap),
            cap,
        }
    }

    /// The wait for this failure, and the wait the failure after it will get.
    pub fn next(&mut self) -> Duration {
        let current = self.next;
        self.next = self.next.saturating_mul(2).min(self.cap);
        current
    }
}
