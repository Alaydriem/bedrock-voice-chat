use std::time::Instant;

/// The mutable, lock-guarded state of a single endpoint's breaker.
pub(super) struct BreakerState {
    pub(super) consecutive_failures: u32,
    pub(super) open_until: Option<Instant>,
    pub(super) open_streak: u32,
    pub(super) half_open: bool,
}

impl BreakerState {
    pub(super) fn new() -> Self {
        Self {
            consecutive_failures: 0,
            open_until: None,
            open_streak: 0,
            half_open: false,
        }
    }
}
