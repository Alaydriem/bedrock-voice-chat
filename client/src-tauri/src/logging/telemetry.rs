use std::sync::atomic::{AtomicBool, Ordering};

pub struct Telemetry {
    state: AtomicBool
}

impl Telemetry {
    pub fn new(state: bool) -> Self {
        Self {
            state: AtomicBool::new(state)
        }
    }

    pub fn set(&self, state: bool) {
        self.state.store(state, Ordering::Relaxed);
    }

    pub fn is_enabled(&self) -> bool {
        self.state.load(Ordering::Relaxed)
    }
}
