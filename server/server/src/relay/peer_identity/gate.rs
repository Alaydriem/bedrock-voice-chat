use std::sync::Arc;
use std::time::Instant;

use common::structs::relay::RelayEndpoint;

use crate::relay::presence::gate::PresenceGate;

use super::store::ServerPeerStore;

// `PresenceGate` backed by the in-memory `ServerPeerStore`. A peer endpoint is
// authorized for a world iff it currently holds a redeemed, unexpired identity
// (within its reconnect grace) bound to exactly that world. This is the live
// gate the `PeerManager` consults for BOTH inbound ingest and outbound
// fan-out; it replaces the fail-closed `NeverProven` stub used while the
// rebuild's identity layer was being built.
pub struct StorePresenceGate {
    store: Arc<ServerPeerStore>,
}

impl StorePresenceGate {
    pub fn new(store: Arc<ServerPeerStore>) -> Self {
        Self { store }
    }

    pub fn new_shared(store: Arc<ServerPeerStore>) -> Arc<Self> {
        Arc::new(Self::new(store))
    }
}

impl PresenceGate for StorePresenceGate {
    fn is_proven(&self, peer: &RelayEndpoint, hashed_world: &str) -> bool {
        // Endpoint key matches `PeerManager::endpoint_key` (`host:port`).
        let endpoint = format!("{}:{}", peer.host, peer.port);
        self.store
            .authorized_world(&endpoint, Instant::now())
            .as_deref()
            == Some(hashed_world)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::ca_cert::CaCertManager;
    use crate::services::CertificateService;
    use std::fs;
    use std::time::Duration;
    use tempfile::TempDir;

    fn store_with_redeemed_peer() -> (Arc<ServerPeerStore>, TempDir) {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().to_str().expect("utf-8 path");
        CaCertManager::new(path)
            .ensure(&[String::from("localhost")])
            .expect("CA");
        let ca_pem = fs::read_to_string(format!("{path}/ca.crt")).expect("ca.crt");
        let cert_service = Arc::new(CertificateService::new(path).expect("cert service"));
        let store = ServerPeerStore::new_shared(cert_service, ca_pem);
        let now = Instant::now();
        let code = store
            .mint("W", "peer.host", 6000, Duration::from_secs(180), now)
            .expect("mint");
        store.redeem(&code, "peer.host:6000", now).expect("redeem");
        (store, dir)
    }

    fn ep(host: &str, port: u16) -> RelayEndpoint {
        RelayEndpoint {
            host: host.into(),
            port,
            primary: false,
        }
    }

    #[test]
    fn authorized_peer_is_proven_for_its_world_only() {
        let (store, _dir) = store_with_redeemed_peer();
        let gate = StorePresenceGate::new(store);
        assert!(
            gate.is_proven(&ep("peer.host", 6000), "W"),
            "a redeemed peer is proven for its bound world"
        );
        assert!(
            !gate.is_proven(&ep("peer.host", 6000), "OTHER"),
            "authorization does not cross worlds"
        );
    }

    #[test]
    fn unknown_peer_is_not_proven() {
        let (store, _dir) = store_with_redeemed_peer();
        let gate = StorePresenceGate::new(store);
        assert!(!gate.is_proven(&ep("stranger.host", 1), "W"));
    }
}
