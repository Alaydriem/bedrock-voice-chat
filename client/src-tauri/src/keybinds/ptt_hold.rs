use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Whether push-to-talk is held, and which edges are real.
///
/// Presses and releases arrive from a global hotkey that repeats, an on-screen button whose
/// gesture can be cancelled, and a controller that may send either one unpaired. Each of
/// those is a way to open a microphone, so the pairing rules are here rather than spread
/// across the three callers:
///
///  - a press while already held changes nothing — key repeat is not a second press
///  - a release with no press behind it changes nothing, so a tap whose press was refused
///    cannot close a microphone it never opened
///  - a press during the release tail reclaims the microphone, so the pending close stands
///    down rather than cutting off a sentence that has already resumed
#[derive(Clone, Default)]
pub struct PttHold {
    held: Arc<AtomicBool>,
}

impl PttHold {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_held(&self) -> bool {
        self.held.load(Ordering::Relaxed)
    }

    /// True when this press is the one that opens the microphone.
    pub fn press(&self) -> bool {
        !self.held.swap(true, Ordering::Relaxed)
    }

    /// True when this release is the one that should start the closing tail.
    pub fn release(&self) -> bool {
        self.held.swap(false, Ordering::Relaxed)
    }

    /// True when a tail that has finished waiting should still close the microphone.
    pub fn tail_should_close(&self) -> bool {
        !self.is_held()
    }

    /// Forget any hold. A voice-mode change resets the microphone either way, so a hold
    /// held across one would otherwise keep a stale claim on it.
    pub fn clear(&self) {
        self.held.store(false, Ordering::Relaxed);
    }
}
