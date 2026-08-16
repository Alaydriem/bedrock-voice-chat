mod verdict;

pub use verdict::CaptureVerdict;

/// Whether the capture device is still delivering, decided from a monotonic frame counter.
///
/// The only thing that noticed a dead microphone before this was cpal's error callback, and a
/// capture stream has more ways to stop than to fail: an endpoint that disappears without
/// raising `StreamError`, an audio focus a phone hands to another app, a callback that stops
/// being scheduled. None of those produce an error, so the stream stayed "running" with a
/// microphone that captured nothing, and the only cure was restarting the application.
///
/// Deliberately not a timer over "did audio arrive". Silence is not death — a muted input and a
/// closed noise gate both capture frames and carry no signal — so this counts frames off the
/// device, which is the one number a stopped capture callback cannot keep moving.
pub struct CaptureWatchdog {
    threshold: u32,
    silent_ticks: u32,
    last_captured: Option<u64>,
}

impl CaptureWatchdog {
    pub fn new(threshold: u32) -> Self {
        Self {
            threshold,
            silent_ticks: 0,
            last_captured: None,
        }
    }

    /// Read the counter once and decide.
    ///
    /// `expected` is whether a session stream is supposed to be capturing right now. While it is
    /// false the watchdog holds no opinion and keeps no history: a stopped stream is not a fault,
    /// and carrying a count across the gap would make the first tick after a restart look like
    /// the tail of the failure that preceded it.
    pub fn observe(&mut self, expected: bool, frames_captured: u64) -> CaptureVerdict {
        if !expected {
            self.rearm(None);
            return CaptureVerdict::Healthy;
        }

        let Some(previous) = self.last_captured else {
            self.rearm(Some(frames_captured));
            return CaptureVerdict::Healthy;
        };

        // A count below the previous one is a counter that was zeroed under us — a new session
        // resets capture accounting — not a device that stopped. Treated as a fresh baseline,
        // because reading it as absent capture would restart a stream that had just started.
        if frames_captured != previous {
            self.rearm(Some(frames_captured));
            return CaptureVerdict::Healthy;
        }

        self.silent_ticks += 1;
        if self.silent_ticks < self.threshold {
            return CaptureVerdict::Quiet;
        }

        // Rearmed on the way out, so the next verdict is a full threshold away. That spacing is
        // the whole backoff: a device that cannot be reopened is retried on a fixed cadence
        // rather than as fast as the loop can run.
        self.rearm(Some(frames_captured));
        CaptureVerdict::Dead
    }

    fn rearm(&mut self, frames_captured: Option<u64>) {
        self.silent_ticks = 0;
        self.last_captured = frames_captured;
    }
}
