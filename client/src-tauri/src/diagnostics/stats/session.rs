use std::sync::Mutex;
use std::time::Instant;

use common::structs::metrics::{ServerId, TransportKind};
use common::structs::reachability::AddressFamily;

#[derive(Debug, Clone)]
struct SessionInfo {
    connected_at: Instant,
    // Absent on a WebSocket session: the family is chosen inside the TLS dialler rather
    // than by a connect walk this can read it from, and reporting a guess would be worse
    // than reporting nothing.
    family: Option<AddressFamily>,
    port: u16,
    transport: TransportKind,
    server: String,
    server_id: String,
}

// What the current connection is, and how long it has been up.
//
// A `Mutex` rather than atomics: written once per connection, read once per second, and never
// from a real-time thread — so the simplest correct thing wins over a pack of atomics that
// could be read mid-update and report a v6 family with a v4 port.
#[derive(Debug, Default)]
pub struct LinkSession {
    info: Mutex<Option<SessionInfo>>,
}

impl LinkSession {
    pub fn new() -> Self {
        Self::default()
    }

    // `family` comes from the winning connect candidate, never from a socket address: a
    // dual-stack socket dials IPv4 destinations as `::ffff:a.b.c.d`, so classifying the
    // address would report every dual-stack client as IPv6.
    pub fn set(
        &self,
        family: Option<AddressFamily>,
        port: u16,
        transport: TransportKind,
        server: String,
        ca_pem: &str,
    ) {
        if let Ok(mut guard) = self.info.lock() {
            *guard = Some(SessionInfo {
                connected_at: Instant::now(),
                family,
                port,
                transport,
                server,
                server_id: ServerId::from_ca_pem(ca_pem.as_bytes()),
            });
        }
    }

    // A stopped stream must stop reporting a port, a family, and a climbing uptime that no
    // longer describe anything.
    pub fn clear(&self) {
        if let Ok(mut guard) = self.info.lock() {
            *guard = None;
        }
    }

    pub fn is_connected(&self) -> bool {
        self.info.lock().map(|g| g.is_some()).unwrap_or(false)
    }

    pub fn uptime_secs(&self) -> u64 {
        self.with(|info| info.connected_at.elapsed().as_secs())
            .unwrap_or(0)
    }

    pub fn family(&self) -> Option<AddressFamily> {
        self.with(|info| info.family).flatten()
    }

    pub fn transport(&self) -> Option<TransportKind> {
        self.with(|info| info.transport)
    }

    pub fn port(&self) -> Option<u16> {
        self.with(|info| info.port)
    }

    pub fn server(&self) -> Option<String> {
        self.with(|info| info.server.clone())
    }

    // The analytics join key. Derived from the CA rather than the hostname so it survives a
    // rename and matches what the server reports for itself.
    pub fn server_id(&self) -> Option<String> {
        self.with(|info| info.server_id.clone())
    }

    fn with<T>(&self, f: impl FnOnce(&SessionInfo) -> T) -> Option<T> {
        self.info.lock().ok().and_then(|guard| guard.as_ref().map(f))
    }
}
