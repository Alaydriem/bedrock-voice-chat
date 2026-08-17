use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// The connected server's chat declaration, as this client last heard it.
///
/// Permissive by default. A client that has not connected, or whose config fetch failed, must
/// behave exactly as it does against a server that has never heard of the switch.
///
/// `Relaxed` because nothing else is ordered against it: a read one tick stale shows a dock
/// that corrects itself on the next poll.
pub struct ChatPolicy {
    enabled: AtomicBool,
}

impl ChatPolicy {
    pub fn new() -> Self {
        Self {
            enabled: AtomicBool::new(true),
        }
    }

    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }
}

impl Default for ChatPolicy {
    fn default() -> Self {
        Self::new()
    }
}
