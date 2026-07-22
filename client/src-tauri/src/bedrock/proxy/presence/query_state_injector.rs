use std::sync::Arc;
use std::time::{Duration, Instant};

use super::pending_query_state::PendingQueryState;

// State that outlives this window is stale — the panel would rather show
// nothing than a snapshot from a dead session.
const BVCS_TTL: Duration = Duration::from_secs(10);

// Producers fire on every local state change whether or not a proxy session is
// running; a bounded queue keeps a proxy-less desktop app from accumulating
// rides forever. Overflow drops the newest message — the TTL already discards
// whatever a late-starting session finds.
const BVCS_QUEUE_CAPACITY: usize = 64;

/// Queue of encoded `!bvcs:` reverse-ride messages (self-state snapshots and
/// per-player preferences) from the `QueryStateReporter` to the proxy session
/// loop, which injects each as a serverbound chat TextPacket. Mirrors the
/// `PresenceInjector` shape. The flume queue is work-stealing, not broadcast:
/// with several concurrent sessions each ride reaches exactly one of them — the
/// desktop app serves a single proxied player, so this never matters outside
/// tests.
pub struct QueryStateInjector {
    tx: flume::Sender<PendingQueryState>,
    rx: flume::Receiver<PendingQueryState>,
}

impl QueryStateInjector {
    pub fn new() -> Self {
        let (tx, rx) = flume::bounded(BVCS_QUEUE_CAPACITY);
        Self { tx, rx }
    }

    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    /// Best-effort enqueue of an already-encoded `!bvcs:` message.
    pub fn enqueue(&self, message: String) {
        let _ = self.tx.try_send(PendingQueryState {
            message,
            deadline: Instant::now() + BVCS_TTL,
        });
    }

    pub fn receiver(&self) -> flume::Receiver<PendingQueryState> {
        self.rx.clone()
    }
}

impl Default for QueryStateInjector {
    fn default() -> Self {
        Self::new()
    }
}
