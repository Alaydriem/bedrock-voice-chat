use tokio::sync::broadcast;

use super::state_signal::ControlStateSignal;

// State changes are human-rate; a small ring is ample, and the reporter treats
// a lagged receiver as "send everything", so overflow only costs one full wave.
const CONTROL_STATE_CAPACITY: usize = 16;

/// Producer half of the control-state reporting plane. Managers fire signals
/// here whenever local audio state changes (mute/deafen/record, per-player
/// gains, connect); the single `QueryStateReporter` consumer debounces them and
/// pushes the resulting state ServerBound. Carries only a plain enum so any
/// binary can construct one without linking the Tauri GUI runtime.
#[derive(Clone)]
pub struct ControlStateBus {
    tx: broadcast::Sender<ControlStateSignal>,
}

impl ControlStateBus {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(CONTROL_STATE_CAPACITY);
        Self { tx }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ControlStateSignal> {
        self.tx.subscribe()
    }

    /// Best-effort signal that self mute/deafen/record changed. A missing
    /// consumer (unit contexts) is fine — the send result is ignored.
    pub fn self_state(&self) {
        let _ = self.tx.send(ControlStateSignal::SelfState);
    }

    /// Best-effort signal that the persisted per-player preferences changed.
    pub fn preferences(&self) {
        let _ = self.tx.send(ControlStateSignal::Preferences);
    }

    /// Best-effort scoped snapshot request from the in-game panel.
    pub fn sync(&self, targets: Vec<String>) {
        let _ = self.tx.send(ControlStateSignal::Sync { targets });
    }
}
