use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

// A held place in the unauthorized-connection budget.
//
// Releases on drop, so every refusal path returns capacity without having to
// remember to — and the one path that forgets cannot leak a slot until restart.
pub struct AdmissionSlot {
    in_flight: Arc<AtomicUsize>,
}

impl AdmissionSlot {
    pub(super) fn new(in_flight: Arc<AtomicUsize>) -> Self {
        Self { in_flight }
    }
}

impl Drop for AdmissionSlot {
    fn drop(&mut self) {
        self.in_flight.fetch_sub(1, Ordering::SeqCst);
    }
}
