use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Collapses a repeating log site down to one line per interval.
///
/// A packet that cannot be serialized fails again on the next tick with the
/// same inputs, so an unguarded `error!` on that path emits at the source's
/// full rate -- a position feed at 4Hz produced ~12,000 identical lines in
/// under an hour, which buries every other line in the log.
pub(crate) struct LogThrottle {
    interval: Duration,
    last_emitted: Mutex<Option<Instant>>,
    suppressed: AtomicU64,
}

impl LogThrottle {
    pub(crate) fn new(interval: Duration) -> Self {
        Self {
            interval,
            last_emitted: Mutex::new(None),
            suppressed: AtomicU64::new(0),
        }
    }

    /// Returns how many occurrences were suppressed since the last emission
    /// when the caller should log now, or `None` when it should stay quiet.
    pub(crate) fn should_log(&self) -> Option<u64> {
        let now = Instant::now();

        let mut last = match self.last_emitted.lock() {
            Ok(last) => last,
            // A poisoned lock only means some other caller panicked while
            // logging; losing throttle state is preferable to propagating.
            Err(poisoned) => poisoned.into_inner(),
        };

        let due = match *last {
            Some(previous) => now.duration_since(previous) >= self.interval,
            None => true,
        };

        if !due {
            self.suppressed.fetch_add(1, Ordering::Relaxed);
            return None;
        }

        *last = Some(now);
        Some(self.suppressed.swap(0, Ordering::Relaxed))
    }
}
