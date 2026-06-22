use std::sync::Arc;
use std::time::{Duration, Instant};

use common::structs::packet::{
    PacketType, PeerPresenceInjectPacket, QuicNetworkPacket, QuicNetworkPacketData,
};
use common::structs::relay::RelayEndpoint;

use super::peer::manager::PeerManager;
use super::peer_identity::ServerPeerStore;

// How often the orchestrator drives the dial-reconcile / idle-sweep cycle. Runs
// off the audio hot path on a dedicated `tokio` task.
const ORCHESTRATION_INTERVAL: Duration = Duration::from_secs(5);

// Delivers a `PeerPresenceInject` to THIS server's own local client(s) currently
// in `hashed_world`, so the client injects a suppressed chat line into the realm.
// Retained for the code-offer path (the minter injects a redemption code through
// its own client); the orchestrator no longer drives presence challenges.
pub trait LocalInjectDelivery: Send + Sync {
    fn deliver_inject(&self, hashed_world: &str, packet: PeerPresenceInjectPacket);
    fn deliver_announce(&self, packet: common::structs::packet::PeerAnnounceInjectPacket);
}

// Sends a code OFFER to a peer (the asker side of Flow 1): asks the peer to mint
// a recipient-bound code for `hashed_world` and inject it into the realm. The
// production impl is an HTTP call via `RelayClient`; the orchestrator drives it
// for each peer `PeerManager::offers_to_send` returns.
pub trait OfferDelivery: Send + Sync {
    fn send_offer(&self, peer: RelayEndpoint, hashed_world: String);
}

// Owns the offer / idle-sweep / reconnect-grace drive loop the runtime spawns.
// It ties the `PeerManager` routing decisions to the live transport without
// touching sockets itself. Peer dials are driven by the observe→redeem path
// (`RedeemedDial`), not by the orchestrator.
pub struct RelayOrchestrator {
    peer_manager: Arc<PeerManager>,
    // Sends code offers for each peer `offers_to_send` returns. When `None`, `tick`
    // computes offers but dispatches none.
    offer_delivery: Option<Arc<dyn OfferDelivery>>,
    // The in-memory identity store, swept each tick so grace-lapsed identities are
    // forgotten (which lets `offers_to_send` re-offer a dropped peer). When `None`
    // the lifecycle is inert.
    server_peer_store: Option<Arc<ServerPeerStore>>,
    interval: Duration,
}

impl RelayOrchestrator {
    pub fn new(peer_manager: Arc<PeerManager>) -> Self {
        Self {
            peer_manager,
            offer_delivery: None,
            server_peer_store: None,
            interval: ORCHESTRATION_INTERVAL,
        }
    }

    // Install the code-offer delivery (the asker side of Flow 1). Without it `tick`
    // dispatches no offers.
    pub fn set_offer_delivery(&mut self, offer_delivery: Arc<dyn OfferDelivery>) {
        self.offer_delivery = Some(offer_delivery);
    }

    // Install the identity store so `tick` drives the reconnect-grace lifecycle:
    // an idle-closed link's identity enters grace, and grace-lapsed identities are
    // swept so a dropped peer is re-offered.
    pub fn set_server_peer_store(&mut self, store: Arc<ServerPeerStore>) {
        self.server_peer_store = Some(store);
    }

    // Override the drive-cycle cadence. Used by the runtime to lower the interval
    // for integration tests; production keeps `ORCHESTRATION_INTERVAL`.
    pub fn set_interval(&mut self, interval: Duration) {
        self.interval = interval;
    }

    // One drive cycle: dispatch code offers, close idle links, and run the
    // reconnect-grace lifecycle. Returns the endpoints whose links went idle so the
    // transport can tear them down.
    pub fn tick(&self, now: Instant) -> Vec<String> {
        // Asker side: offer a code to each discovered, unauthorized peer we should
        // initiate to. The peer mints + injects it; our client observes + redeems,
        // and the observe path (`RedeemedDial`) opens the link.
        if let Some(offer) = &self.offer_delivery {
            for (world, peer) in self.peer_manager.offers_to_send(now) {
                let key = PeerManager::endpoint_key(&peer);
                offer.send_offer(peer, world.clone());
                self.peer_manager.record_offer(&key, &world, now);
            }
        }

        let closed = self.peer_manager.sweep_idle(now);

        self.peer_manager.sweep_announced_peers(now);

        // Reconnect-grace lifecycle: an idle-closed link's identity enters
        // grace (a prompt reconnect revalidates without a fresh code); grace-lapsed
        // identities are forgotten so `offers_to_send` re-offers the peer.
        if let Some(store) = &self.server_peer_store {
            for endpoint in &closed {
                store.mark_disconnected(endpoint, now, ServerPeerStore::RECONNECT_GRACE);
            }
            store.sweep(now);
        }

        closed
    }

    pub async fn run(self) {
        let mut ticker = tokio::time::interval(self.interval);
        loop {
            ticker.tick().await;
            let closed = self.tick(Instant::now());
            for endpoint in closed {
                tracing::info!(
                    "relay peer link {} idle-closed (bilateral teardown)",
                    endpoint
                );
            }
        }
    }

    // Builds the server→client `PeerPresenceInject` QUIC packet for a token. Kept
    // here so both the production sink and tests construct the identical wire
    // packet; reused by the code-offer path to carry a redemption code.
    pub fn inject_packet(packet: PeerPresenceInjectPacket) -> QuicNetworkPacket {
        QuicNetworkPacket {
            packet_type: PacketType::PeerPresenceInject,
            owner: None,
            data: QuicNetworkPacketData::PeerPresenceInject(packet),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::relay::peer::link::ingest_sink::RelayIngestSink;
    use crate::relay::peer::table::PeerTable;
    use crate::relay::presence::gate::NeverProven;
    use common::structs::packet::QuicNetworkPacket;
    use common::structs::relay::RelayEndpoint;
    use std::sync::Mutex as StdMutex;

    fn ep(host: &str, port: u16) -> RelayEndpoint {
        RelayEndpoint {
            host: host.into(),
            port,
            primary: false,
        }
    }

    struct NoopSink;
    #[async_trait::async_trait]
    impl RelayIngestSink for NoopSink {
        async fn publish(&self, _packet: QuicNetworkPacket) {}
    }

    #[derive(Default)]
    struct CaptureOffer {
        offered: StdMutex<Vec<(String, String)>>,
    }
    impl OfferDelivery for CaptureOffer {
        fn send_offer(&self, peer: RelayEndpoint, hashed_world: String) {
            self.offered
                .lock()
                .unwrap()
                .push((format!("{}:{}", peer.host, peer.port), hashed_world));
        }
    }

    // The asker side: `tick` offers a code to each discovered, unauthorized peer
    // it should initiate to.
    #[test]
    fn tick_dispatches_offers_to_unauthorized_initiator_peers() {
        let table = PeerTable::new_shared();
        table.set_active_worlds(vec!["W".into()]);
        table.set_world_peers("W", vec![ep("b", 1)]);
        // NeverProven: the peer is unauthorized -> we should offer.
        let mgr =
            PeerManager::new_shared(ep("a", 1), table, Arc::new(NoopSink), Arc::new(NeverProven));
        let mut orch = RelayOrchestrator::new(mgr);
        let offer = Arc::new(CaptureOffer::default());
        orch.set_offer_delivery(offer.clone());
        orch.tick(Instant::now());
        assert_eq!(
            offer.offered.lock().unwrap().clone(),
            vec![("b:1".to_string(), "W".to_string())]
        );
    }

    #[test]
    fn idle_close_starts_grace_then_sweep_forgets_the_identity() {
        use crate::relay::peer_identity::ServerPeerStore;
        use crate::runtime::ca_cert::CaCertManager;
        use crate::services::CertificateService;
        use std::time::Duration;
        use tempfile::TempDir;

        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().to_str().expect("utf-8 path");
        CaCertManager::new(path)
            .ensure(&[String::from("localhost")])
            .expect("CA");
        let ca_pem = std::fs::read_to_string(format!("{path}/ca.crt")).expect("ca.crt");
        let cert_service = Arc::new(CertificateService::new(path).expect("cert service"));
        let store = ServerPeerStore::new_shared(cert_service, ca_pem);

        let table = PeerTable::new_shared();
        let mgr =
            PeerManager::new_shared(ep("a", 1), table, Arc::new(NoopSink), Arc::new(NeverProven));
        let mut orch = RelayOrchestrator::new(mgr.clone());
        orch.set_server_peer_store(store.clone());

        // A redeemed peer with an idle inbound link.
        store.authorize_peer("b:1", "W");
        let t0 = Instant::now();
        mgr.register_inbound("b:1", t0);

        // Idle-close fires past the idle timeout: the identity enters reconnect
        // grace (still authorized so a prompt reconnect needs no fresh code).
        let after_idle = t0 + Duration::from_secs(301);
        let closed = orch.tick(after_idle);
        assert_eq!(closed, vec!["b:1".to_string()]);
        assert_eq!(
            store.authorized_world("b:1", after_idle).as_deref(),
            Some("W"),
            "within grace the dropped peer stays authorized"
        );

        // Past the grace deadline a later tick sweeps the identity, so the peer
        // would be re-offered rather than silently relayed to.
        let after_grace = after_idle + Duration::from_secs(31);
        orch.tick(after_grace);
        assert_eq!(
            store.authorized_world("b:1", after_grace),
            None,
            "past the reconnect grace the identity is forgotten"
        );
    }

    #[test]
    fn inject_packet_carries_the_token() {
        let p = RelayOrchestrator::inject_packet(PeerPresenceInjectPacket {
            token: "tok".into(),
            ttl_ms: 5000,
        });
        assert_eq!(p.packet_type, PacketType::PeerPresenceInject);
        match p.data {
            QuicNetworkPacketData::PeerPresenceInject(inner) => assert_eq!(inner.token, "tok"),
            _ => panic!("expected PeerPresenceInject"),
        }
    }
}
