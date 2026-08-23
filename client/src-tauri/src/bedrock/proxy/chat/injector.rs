use std::sync::Arc;
use std::time::{Duration, Instant};

use super::pending_send::PendingChatSend;

/// A line the sender has stopped expecting to land. Long enough to cover a brief reconnect,
/// short enough that nobody is surprised by their message appearing later.
const SEND_TTL: Duration = Duration::from_secs(15);

/// Bounded so a proxy-less desktop app cannot accumulate messages forever. Overflow drops the
/// newest, which the composer surfaces as a failed send.
const SEND_QUEUE_CAPACITY: usize = 32;

/// Queue of app-composed chat lines from the Tauri command layer to the proxy session loop,
/// which injects each as a serverbound chat `TextPacket` under the player's own name.
///
/// Mirrors `QueryStateInjector`. The flume queue is work-stealing rather than broadcast: with
/// several concurrent sessions each line reaches exactly one, and the desktop app serves a
/// single proxied player.
pub struct ChatInjector {
    tx: flume::Sender<PendingChatSend>,
    rx: flume::Receiver<PendingChatSend>,
}

impl ChatInjector {
    pub fn new() -> Self {
        let (tx, rx) = flume::bounded(SEND_QUEUE_CAPACITY);
        Self { tx, rx }
    }

    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    /// `false` when the queue is full, which the caller reports rather than swallowing — a
    /// message that silently vanishes is worse than one that says it failed.
    pub fn enqueue(&self, text: String) -> bool {
        self.tx
            .try_send(PendingChatSend {
                text,
                deadline: Instant::now() + SEND_TTL,
            })
            .is_ok()
    }

    pub fn receiver(&self) -> flume::Receiver<PendingChatSend> {
        self.rx.clone()
    }
}

impl Default for ChatInjector {
    fn default() -> Self {
        Self::new()
    }
}
