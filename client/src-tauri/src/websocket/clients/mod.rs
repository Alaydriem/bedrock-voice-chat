use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use common::structs::websocket::WebSocketClientInfo;

// Every connected WebSocket client, keyed by a handle: two clients may report one name.
pub struct WebSocketClients {
    entries: Mutex<BTreeMap<u64, WebSocketClientInfo>>,
    next: AtomicU64,
}

impl WebSocketClients {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(BTreeMap::new()),
            next: AtomicU64::new(1),
        }
    }

    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    pub fn register(&self, name: &str, route: &str) -> u64 {
        let id = self.next.fetch_add(1, Ordering::SeqCst);
        let trimmed = name.trim();
        let info = WebSocketClientInfo {
            id: id.to_string(),
            name: if trimmed.is_empty() {
                "Unnamed client".to_string()
            } else {
                trimmed.to_string()
            },
            route: route.to_string(),
            connected_at: Self::now(),
            commands: 0,
        };
        if let Ok(mut entries) = self.entries.lock() {
            entries.insert(id, info);
        }
        id
    }

    pub fn count_command(&self, id: u64) {
        if let Ok(mut entries) = self.entries.lock()
            && let Some(entry) = entries.get_mut(&id)
        {
            entry.commands = entry.commands.saturating_add(1);
        }
    }

    pub fn release(&self, id: u64) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.remove(&id);
        }
    }

    pub fn snapshot(&self) -> Vec<WebSocketClientInfo> {
        self.entries
            .lock()
            .map(|entries| entries.values().cloned().collect())
            .unwrap_or_default()
    }

    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
}

impl Default for WebSocketClients {
    fn default() -> Self {
        Self::new()
    }
}

mod registration;

pub use registration::ClientRegistration;
