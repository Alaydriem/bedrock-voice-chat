use common::structs::control::ClientActionType;
use log::debug;

// Control actions are low-rate, human-initiated events; a small bound is ample
// and keeps a stalled consumer from buffering unbounded state flips.
const CONTROL_ACTION_CAPACITY: usize = 64;

/// Producer half of the control-action plane. Proxy sessions and the QUIC output
/// router push delivered `ClientActionType`s through this sender; the single
/// consumer (`ControlActionsManager::run`) owns the `AppHandle` and applies them
/// against the desktop managers.
///
/// This indirection is deliberate: holding a `ControlActionsManager` (which embeds
/// an `AppHandle`) as a struct field links the entire Tauri GUI runtime into any
/// binary that constructs the struct — including cargo test binaries, which then
/// fail to load on Windows (comctl32 v6 imports without an application manifest).
/// A sender of a plain DTO carries no such linkage; tests construct one with
/// `ControlActionSender::channel()` and drop or inspect the receiver.
#[derive(Clone)]
pub struct ControlActionSender {
    tx: flume::Sender<ClientActionType>,
}

impl ControlActionSender {
    pub fn new(tx: flume::Sender<ClientActionType>) -> Self {
        Self { tx }
    }

    /// Create a bounded control-action channel and the sender wrapping it.
    pub fn channel() -> (Self, flume::Receiver<ClientActionType>) {
        let (tx, rx) = flume::bounded(CONTROL_ACTION_CAPACITY);
        (Self::new(tx), rx)
    }

    /// Best-effort, non-blocking send. A missing consumer (unit contexts) or a
    /// full queue drops the action; control actions are idempotent state flips,
    /// so dropping under backpressure is preferable to blocking a packet path.
    pub fn send(&self, action: ClientActionType) {
        if let Err(e) = self.tx.try_send(action) {
            debug!("ControlActionSender: dropping control action: {e}");
        }
    }
}
