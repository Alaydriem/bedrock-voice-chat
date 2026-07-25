use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use log::Record;
use moka::sync::Cache;

const THROTTLE_WINDOW: Duration = Duration::from_secs(30);
const THROTTLE_CAPACITY: u64 = 512;

pub(crate) enum ThrottleDecision {
    Emit { suppressed: u32 },
    Suppress,
}

struct WindowState {
    window_start: Instant,
    emitted: bool,
    suppressed: u32,
}

pub(crate) struct LogThrottle {
    window: Duration,
    states: Cache<u64, Arc<Mutex<WindowState>>>,
}

impl LogThrottle {
    pub(crate) fn new() -> Self {
        Self::with(THROTTLE_WINDOW, THROTTLE_CAPACITY)
    }

    fn with(window: Duration, capacity: u64) -> Self {
        Self {
            window,
            states: Cache::builder()
                .time_to_idle(window.saturating_mul(4))
                .max_capacity(capacity)
                .build(),
        }
    }

    pub(crate) fn window_secs(&self) -> u64 {
        self.window.as_secs()
    }

    /// Decide whether an error record should reach Sentry. The first occurrence of
    /// a fingerprint in a window is emitted; identical records within the same
    /// window are dropped and counted, and the count rides along on the next emit.
    pub(crate) fn evaluate(&self, record: &Record) -> ThrottleDecision {
        let fingerprint = Self::fingerprint(record);
        let now = Instant::now();

        let state = self.states.get_with(fingerprint, || {
            Arc::new(Mutex::new(WindowState {
                window_start: now,
                emitted: false,
                suppressed: 0,
            }))
        });

        let mut state = state.lock().unwrap();

        if now.duration_since(state.window_start) >= self.window {
            let suppressed = state.suppressed;
            state.window_start = now;
            state.emitted = true;
            state.suppressed = 0;
            return ThrottleDecision::Emit { suppressed };
        }

        if !state.emitted {
            state.emitted = true;
            return ThrottleDecision::Emit { suppressed: 0 };
        }

        state.suppressed = state.suppressed.saturating_add(1);
        ThrottleDecision::Suppress
    }

    fn fingerprint(record: &Record) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        record.target().hash(&mut hasher);
        record.level().hash(&mut hasher);
        Self::hash_normalized(&record.args().to_string(), &mut hasher);
        hasher.finish()
    }

    // Collapse runs of digits to a single marker so that timestamps, ports,
    // request ids and numeric error details from an otherwise identical message
    // share one fingerprint instead of each looking unique.
    fn hash_normalized(message: &str, hasher: &mut impl Hasher) {
        let mut prev_digit = false;
        for c in message.chars() {
            if c.is_ascii_digit() {
                if !prev_digit {
                    '#'.hash(hasher);
                    prev_digit = true;
                }
            } else {
                c.hash(hasher);
                prev_digit = false;
            }
        }
    }
}
