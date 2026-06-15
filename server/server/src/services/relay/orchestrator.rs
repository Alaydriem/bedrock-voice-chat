use std::sync::Arc;
use std::time::{Duration, Instant};

use common::structs::packet::{
    PacketType, PeerPresenceInjectPacket, QuicNetworkPacket, QuicNetworkPacketData,
};

use super::peer_dial_driver::PeerDialDriver;
use super::peer_manager::PeerManager;

// How often the orchestrator drives the presence-challenge / echo / idle-sweep
// cycle. Independent of the relay register/lookup interval; both run off the
// audio hot path on dedicated `tokio` tasks.
const ORCHESTRATION_INTERVAL: Duration = Duration::from_secs(5);

// Delivers a `PeerPresenceInject` to THIS server's own local client(s) currently
// in `hashed_world`, so the client injects the suppressed `!bvcp <token>` chat
// into the realm. This is the only path a challenge token travels — never to a
// peer link.
pub trait LocalInjectDelivery: Send + Sync {
    fn deliver_inject(&self, hashed_world: &str, packet: PeerPresenceInjectPacket);
}

// Sends a `PeerPresenceObserved` echo to the relevant peer link(s) — tokens OUR
// client observed in the realm, echoed back so the peer can complete its proof
// of us. The token is world-attributed so the implementation records our
// mutual-proof half against the correct world.
pub trait PeerEchoDelivery: Send + Sync {
    fn echo_observed(&self, token: &str, hashed_world: &str);
}

// Owns the presence-proof / idle-sweep drive loop that the runtime spawns. It
// ties the `PeerManager` routing decisions to the live transport-adjacent
// delivery seams (`LocalInjectDelivery`, `PeerEchoDelivery`) without itself
// touching sockets, so the full cycle is exercisable in-process.
pub struct RelayOrchestrator {
    peer_manager: Arc<PeerManager>,
    inject: Arc<dyn LocalInjectDelivery>,
    echo: Arc<dyn PeerEchoDelivery>,
    // Drives a peer dial for each intent `reconcile` produces. When `None`,
    // `tick` still calls `reconcile` to keep the link table current but performs
    // no dial.
    dial_driver: Option<Arc<dyn PeerDialDriver>>,
    interval: Duration,
}

impl RelayOrchestrator {
    pub fn new(
        peer_manager: Arc<PeerManager>,
        inject: Arc<dyn LocalInjectDelivery>,
        echo: Arc<dyn PeerEchoDelivery>,
    ) -> Self {
        Self {
            peer_manager,
            inject,
            echo,
            dial_driver: None,
            interval: ORCHESTRATION_INTERVAL,
        }
    }

    pub fn new_with_driver(
        peer_manager: Arc<PeerManager>,
        inject: Arc<dyn LocalInjectDelivery>,
        echo: Arc<dyn PeerEchoDelivery>,
        dial_driver: Arc<dyn PeerDialDriver>,
    ) -> Self {
        Self {
            peer_manager,
            inject,
            echo,
            dial_driver: Some(dial_driver),
            interval: ORCHESTRATION_INTERVAL,
        }
    }

    // One drive cycle: emit any needed presence challenges to local clients,
    // drain observed tokens to echo back to peers, reconcile dial intentions,
    // and close idle links. Returns the endpoints whose links went idle so the
    // transport can tear them down bilaterally.
    pub fn tick(&self, now: Instant) -> Vec<String> {
        for (world, packet) in self.peer_manager.challenges_to_send(now) {
            self.inject.deliver_inject(&world, packet);
        }

        for (token, world) in self.peer_manager.tokens_to_echo_to_peer() {
            self.echo.echo_observed(&token, &world);
        }

        // Consume (do NOT discard) the dial intents reconcile
        // produced: for each peer we should initiate to, kick off a real dial via
        // the driver, scoped to the shared world for the peer-cert fetch.
        let dial_intents = self.peer_manager.reconcile(now);
        if let Some(driver) = &self.dial_driver {
            for peer_ep in dial_intents {
                match self.peer_manager.world_for_peer(&peer_ep) {
                    Some(world) => driver.begin_dial(peer_ep, world),
                    None => tracing::debug!(
                        "relay: dial intent for {} has no active shared world; skipping",
                        peer_ep
                    ),
                }
            }
        }

        self.peer_manager.sweep_idle(now)
    }

    pub async fn run(self) {
        let mut ticker = tokio::time::interval(self.interval);
        loop {
            ticker.tick().await;
            let closed = self.tick(Instant::now());
            for endpoint in closed {
                tracing::info!("relay peer link {} idle-closed (bilateral teardown)", endpoint);
            }
        }
    }
}

impl RelayOrchestrator {
    // Builds the server→client `PeerPresenceInject` QUIC packet for a token. Kept
    // here (not in the delivery impl) so both the production sink and tests
    // construct the identical wire packet.
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
    use crate::services::relay::peer_table::PeerTable;
    use crate::services::relay::presence::PresenceProver;
    use crate::services::relay::ingest_sink::RelayIngestSink;
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
    struct CaptureInject {
        injected: StdMutex<Vec<(String, String)>>,
    }
    impl LocalInjectDelivery for CaptureInject {
        fn deliver_inject(&self, hashed_world: &str, packet: PeerPresenceInjectPacket) {
            self.injected
                .lock()
                .unwrap()
                .push((hashed_world.to_string(), packet.token));
        }
    }

    #[derive(Default)]
    struct CaptureEcho {
        echoed: StdMutex<Vec<(String, String)>>,
    }
    impl PeerEchoDelivery for CaptureEcho {
        fn echo_observed(&self, token: &str, hashed_world: &str) {
            self.echoed
                .lock()
                .unwrap()
                .push((token.to_string(), hashed_world.to_string()));
        }
    }

    #[test]
    fn tick_injects_challenge_for_unproven_peer() {
        let table = PeerTable::new_shared();
        table.set_active_worlds(vec!["W".into()]);
        table.set_world_peers("W", vec![ep("b", 1)]);
        let prover = PresenceProver::new_shared();
        let mgr = PeerManager::new_shared(ep("a", 1), table, Arc::new(NoopSink), prover.clone());
        mgr.set_prover(prover);

        let inject = Arc::new(CaptureInject::default());
        let echo = Arc::new(CaptureEcho::default());
        let orch = RelayOrchestrator::new(mgr, inject.clone(), echo);
        orch.tick(Instant::now());

        let injected = inject.injected.lock().unwrap();
        assert_eq!(injected.len(), 1);
        assert_eq!(injected[0].0, "W");
        assert_eq!(injected[0].1.len(), 32);
    }

    #[test]
    fn tick_echoes_observed_tokens_to_peer() {
        let table = PeerTable::new_shared();
        let prover = PresenceProver::new_shared();
        let mgr = PeerManager::new_shared(ep("a", 1), table, Arc::new(NoopSink), prover.clone());
        mgr.set_prover(prover);
        let now = Instant::now();
        // the observed token must be a known (expected) challenge
        mgr.expect_observed("peer-token", "W", now);
        mgr.on_local_client_observed("peer-token", now);

        let inject = Arc::new(CaptureInject::default());
        let echo = Arc::new(CaptureEcho::default());
        let orch = RelayOrchestrator::new(mgr, inject, echo.clone());
        orch.tick(now);

        assert_eq!(
            echo.echoed.lock().unwrap().clone(),
            vec![("peer-token".to_string(), "W".to_string())]
        );
    }

    // Reconcile's dial intents must be CONSUMED, not discarded. With
    // a dial driver installed, a proven peer this server should initiate to is
    // handed to the driver (scoped to the shared world).
    #[derive(Default)]
    struct CaptureDial {
        dialed: StdMutex<Vec<(String, String)>>,
    }
    impl crate::services::relay::peer_dial_driver::PeerDialDriver for CaptureDial {
        fn begin_dial(&self, peer_ep: String, hashed_world: String) {
            self.dialed.lock().unwrap().push((peer_ep, hashed_world));
        }
    }

    #[test]
    fn tick_consumes_reconcile_dial_intents() {
        use crate::services::relay::presence::PresenceProver;
        let table = PeerTable::new_shared();
        table.set_active_worlds(vec!["W".into()]);
        // self is "a:1"; peer "b:1" is lexically higher -> we initiate to it.
        table.set_world_peers("W", vec![ep("b", 1)]);
        let prover = PresenceProver::new_shared();
        let mgr = PeerManager::new_shared(ep("a", 1), table, Arc::new(NoopSink), prover.clone());
        mgr.set_prover(prover.clone());

        // Complete the mutual proof for W against b:1 so reconcile will dial it.
        let now = Instant::now();
        let token = prover.new_challenge("W", now);
        prover.record_observed_from_peer("b:1", &token, now);
        prover.record_echoed_to_peer("b:1", "W");

        let driver = Arc::new(CaptureDial::default());
        let orch = RelayOrchestrator::new_with_driver(
            mgr,
            Arc::new(CaptureInject::default()),
            Arc::new(CaptureEcho::default()),
            driver.clone(),
        );
        orch.tick(now);

        assert_eq!(
            driver.dialed.lock().unwrap().clone(),
            vec![("b:1".to_string(), "W".to_string())],
            "reconcile dial intents must be acted on, not discarded"
        );
    }

    // A packet enqueued onto a link's outbound queue is taken by the
    // (stub) drain — the receiver `take_outbound_receiver` hands to the writer.
    #[tokio::test]
    async fn outbound_enqueued_packet_is_taken_by_drain() {
        use crate::services::relay::presence_gate::AlwaysProven;
        use crate::services::relay::relayed_packet::RelayedPacket;
        use common::structs::packet::{AudioFramePacket, PacketType, QuicNetworkPacketData};

        let table = PeerTable::new_shared();
        table.set_active_worlds(vec!["W".into()]);
        table.set_world_peers("W", vec![ep("b", 1)]);
        let mgr = PeerManager::new_shared(ep("a", 1), table, Arc::new(NoopSink), Arc::new(AlwaysProven));
        let now = Instant::now();
        // Establish a link (acceptor) for the peer so forward_local can enqueue.
        mgr.register_inbound("b:1", now);

        // The drain (write pump) takes the receiver, just as the dialer would.
        let mut rx = mgr
            .take_outbound_receiver("b:1")
            .expect("link should hand off its outbound receiver once");

        let audio = QuicNetworkPacket {
            packet_type: PacketType::AudioFrame,
            owner: None,
            data: QuicNetworkPacketData::AudioFrame(AudioFramePacket::new(
                vec![1, 2, 3],
                48000,
                None,
                Some(true),
            )),
        };
        let sent = mgr.forward_local(&RelayedPacket::local(audio), "W");
        assert_eq!(sent, 1, "one copy enqueued to the single peer");

        // The enqueued packet is drained off the queue (reaches the writer).
        let drained = rx.recv().await;
        assert!(drained.is_some(), "the enqueued outbound packet must be taken by the drain");

        // The receiver is takeable only once.
        assert!(mgr.take_outbound_receiver("b:1").is_none());
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
