use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Shared readiness flags set by long-lived components and read by the
/// /health/readiness route. SeqCst: readiness gates K8s traffic admission,
/// which is critical-flag territory.
pub struct ReadinessState {
    quic_ready: AtomicBool,
}

impl ReadinessState {
    pub fn new() -> Self {
        Self {
            quic_ready: AtomicBool::new(false),
        }
    }

    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    pub fn set_quic_ready(&self, ready: bool) {
        self.quic_ready.store(ready, Ordering::SeqCst);
    }

    pub fn quic_ready(&self) -> bool {
        self.quic_ready.load(Ordering::SeqCst)
    }
}

impl Default for ReadinessState {
    fn default() -> Self {
        Self::new()
    }
}
