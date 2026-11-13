use common::structs::network::ServerAddress;
use std::collections::HashSet;
use std::sync::{Arc, RwLock};
use tauri_plugin_curia::curia;

/// Which servers have proven QUIC unusable for this run.
///
/// Records what `ReachabilityProbe` cannot: a network that completes the handshake and
/// then degrades the session probes reachable forever.
///
/// A demotion is permanent for the run — no expiry, no re-probe. Only an application
/// restart clears it. Keyed on host.
pub struct TransportVerdict {
    demoted: RwLock<HashSet<String>>,
}

impl TransportVerdict {
    pub fn new() -> Self {
        Self {
            demoted: RwLock::new(HashSet::new()),
        }
    }

    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    /// Records that QUIC failed this host in a way a reachability probe cannot see.
    pub fn demote(&self, server: &str) {
        let host = Self::host_of(server);
        let inserted = self
            .demoted
            .write()
            .map(|mut demoted| demoted.insert(host.clone()))
            .unwrap_or(false);

        if inserted {
            curia::warn!("QUIC demoted; the WebSocket transport is preferred for the rest of this run", {
                defect: crate::logging::Defect::TransportFellBack,
                transport: "wss",
                connected_server: host.clone(),
            });
        }
    }

    pub fn is_demoted(&self, server: &str) -> bool {
        let host = Self::host_of(server);
        self.demoted
            .read()
            .map(|demoted| demoted.contains(&host))
            .unwrap_or(false)
    }

    /// Reduces a bare FQDN or a full URL to one key.
    ///
    /// Selection holds the FQDN, the health monitor holds the URL. Normalizing here stops
    /// a demotion being written under one spelling and read under another.
    fn host_of(server: &str) -> String {
        match ServerAddress::parse(server) {
            Ok(address) => address.host().to_ascii_lowercase(),
            Err(_) => server.to_ascii_lowercase(),
        }
    }
}

impl Default for TransportVerdict {
    fn default() -> Self {
        Self::new()
    }
}
