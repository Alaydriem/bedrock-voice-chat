use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

/// The canonical identity of the active QUIC connection, `game:gamertag`.
/// `NetworkStreamManager` publishes it when a stream comes up and clears it on stop/reset.
///
/// The `QueryStateReporter` stamps `QueryState.id` / `PlayerPreference.owner` with it, and
/// the server drops any report whose id differs from the identity it read off the
/// certificate — so a bare gamertag here silently discards every control report.
///
/// The client stamps nothing on the wire. This exists so the client can name itself in the
/// payloads it sends, not to assert who it is.
pub struct ConnectionIdentity {
    name: RwLock<Option<String>>,
    generation: AtomicU64,
}

impl ConnectionIdentity {
    pub fn new() -> Self {
        Self {
            name: RwLock::new(None),
            generation: AtomicU64::new(0),
        }
    }

    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    pub fn set(&self, name: Option<String>) {
        if let Ok(mut guard) = self.name.write() {
            *guard = name;
        }
        self.generation.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get(&self) -> Option<String> {
        self.name.read().ok().and_then(|guard| guard.clone())
    }

    /// Which connection the identity belongs to.
    ///
    /// Changes on every `set`, so a reconnect under the same name still reads as a new
    /// connection. The server evicts a player's preferences when their connection closes, and a
    /// reporter that compared names alone would not send them again.
    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::Relaxed)
    }
}
