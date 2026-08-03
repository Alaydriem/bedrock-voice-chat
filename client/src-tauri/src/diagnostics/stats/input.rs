use std::sync::atomic::{AtomicU64, Ordering};

// Capture-side accounting, written from the CPAL callback. Atomics only: that thread has a
// hard deadline and must not allocate or block.
//
// There is no "gate open" flag here on purpose. The gate's state is inferred from whether any
// frame in an interval carried signal, because at a 20 ms frame cadence a single instantaneous
// reading lands on a near-random frame and flickers between open and closed.
#[derive(Debug, Default)]
pub struct InputPipelineStats {
    frames_captured: AtomicU64,
    frames_with_signal: AtomicU64,
    frames_sent: AtomicU64,
}

impl InputPipelineStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_frame(&self, is_silent: bool) {
        self.frames_captured.fetch_add(1, Ordering::Relaxed);
        if !is_silent {
            self.frames_with_signal.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn record_sent(&self) {
        self.frames_sent.fetch_add(1, Ordering::Relaxed);
    }

    /// Zero every counter.
    ///
    /// The metering stream on the setup screen captures frames and sends none of them,
    /// so leaving its totals in place would have a session open on a diagnostic that
    /// already reports thousands of captured frames against zero sent — the exact
    /// signature of a broken encoder.
    pub fn reset(&self) {
        self.frames_captured.store(0, Ordering::Relaxed);
        self.frames_with_signal.store(0, Ordering::Relaxed);
        self.frames_sent.store(0, Ordering::Relaxed);
    }

    pub fn frames_captured(&self) -> u64 {
        self.frames_captured.load(Ordering::Relaxed)
    }

    pub fn frames_with_signal(&self) -> u64 {
        self.frames_with_signal.load(Ordering::Relaxed)
    }

    pub fn frames_sent(&self) -> u64 {
        self.frames_sent.load(Ordering::Relaxed)
    }
}
