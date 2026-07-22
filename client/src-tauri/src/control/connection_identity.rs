use std::sync::{Arc, RwLock};

/// The gamertag identity of the active QUIC connection. `NetworkStreamManager`
/// publishes it when a stream comes up and clears it on stop/reset. The
/// `QueryStateReporter` stamps `QueryState.id` / `PlayerPreference.owner` with
/// it; the server drops reports whose id differs from the connection author, so
/// this must be exactly the name the network `OutputStream` stamps as the
/// packet owner.
pub struct ConnectionIdentity {
    name: RwLock<Option<String>>,
}

impl ConnectionIdentity {
    pub fn new() -> Self {
        Self {
            name: RwLock::new(None),
        }
    }

    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    pub fn set(&self, name: Option<String>) {
        if let Ok(mut guard) = self.name.write() {
            *guard = name;
        }
    }

    pub fn get(&self) -> Option<String> {
        self.name.read().ok().and_then(|guard| guard.clone())
    }
}
