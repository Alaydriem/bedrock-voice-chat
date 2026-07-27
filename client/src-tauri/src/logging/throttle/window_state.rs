use std::time::Instant;

/// Per-fingerprint window: whether this window has already emitted, and how many
/// identical records were suppressed while it was open.
pub(super) struct WindowState {
    pub(super) window_start: Instant,
    pub(super) emitted: bool,
    pub(super) suppressed: u32,
}

impl WindowState {
    pub(super) fn new(window_start: Instant) -> Self {
        Self {
            window_start,
            emitted: false,
            suppressed: 0,
        }
    }
}
