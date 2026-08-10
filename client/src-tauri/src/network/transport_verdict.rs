use std::collections::HashSet;
use std::sync::{Arc, RwLock};

/// Which servers have proven QUIC unusable for this run.
///
/// `ReachabilityProbe` answers "did a handshake complete", which is the wrong question on
/// a network that permits the handshake and then degrades the session. Such a network
/// probes reachable forever, so probe-driven selection loops: connect, degrade, tear down,
/// re-probe, connect again. This records the verdict the probe cannot reach.
///
/// **A demotion is permanent for the run.** No expiry, no re-probe, no promotion back to
/// QUIC — only restarting the application clears it. A timer would return a player to a
/// transport already shown to break here, and the failure that follows is a silent audio
/// outage rather than an error. Staying demoted costs latency on a transport that works.
///
/// Keyed on host, so connecting to a different server still judges that server on its own
/// evidence.
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
            log::warn!(
                "QUIC demoted for {host}: the WebSocket transport will be preferred for the rest of this run"
            );
        }
    }

    pub fn is_demoted(&self, server: &str) -> bool {
        let host = Self::host_of(server);
        self.demoted
            .read()
            .map(|demoted| demoted.contains(&host))
            .unwrap_or(false)
    }

    /// Reduces anything that names a server — a bare FQDN, or a URL with a scheme, port
    /// and path — to one key.
    ///
    /// The two callers hold different forms: selection knows the FQDN, the health monitor
    /// knows the URL it was told to probe. Normalizing here rather than at each call site
    /// is what stops a demotion being written under one spelling and read under another,
    /// which would look exactly like the demotion never happening.
    fn host_of(server: &str) -> String {
        let without_scheme = server
            .split_once("://")
            .map(|(_, rest)| rest)
            .unwrap_or(server);
        let without_path = without_scheme
            .split(['/', '?'])
            .next()
            .unwrap_or(without_scheme);

        // An IPv6 literal is bracketed, so the last colon only separates a port when it
        // falls outside the brackets.
        let host = match without_path.rfind(']') {
            Some(end) => &without_path[..=end],
            None => without_path
                .split_once(':')
                .map(|(host, _)| host)
                .unwrap_or(without_path),
        };

        host.to_ascii_lowercase()
    }
}

impl Default for TransportVerdict {
    fn default() -> Self {
        Self::new()
    }
}
