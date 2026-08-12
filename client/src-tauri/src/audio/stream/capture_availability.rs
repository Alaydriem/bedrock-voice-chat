use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Whether the capture device can be opened at all.
///
/// False only after every rebuild attempt has been spent. Read by the runtime-state poll, which
/// runs once a second on the webview's own schedule, so it is an atomic rather than anything the
/// audio manager's lock guards: a rebuild can hold that lock for seconds, and the poll must not
/// wait behind it to report the failure the rebuild just produced.
pub struct CaptureAvailability {
    available: AtomicBool,
}

impl CaptureAvailability {
    pub fn new() -> Self {
        Self {
            available: AtomicBool::new(true),
        }
    }

    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    pub fn set(&self, available: bool) {
        self.available.store(available, Ordering::Relaxed);
    }

    pub fn get(&self) -> bool {
        self.available.load(Ordering::Relaxed)
    }
}

impl Default for CaptureAvailability {
    fn default() -> Self {
        Self::new()
    }
}
