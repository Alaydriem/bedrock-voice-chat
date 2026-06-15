// In-process end-to-end wiring test for the cross-server voice relay.
//
// It stands up TWO relay setups, A and B, sharing a `relay_world_uuid` ("W"),
// with stubbed "local clients" that hand-deliver injected `!bvcp` tokens between
// the realms (simulating the realm fan-out). It then drives the full mutual
// presence handshake and asserts:
//   1. After the mutual handshake, the gate authorizes peering both ways.
//   2. A LOCAL audio packet on A is forwarded to B and ingested into B's
//      broadcast pipeline WITHOUT going through player registration (a spy
//      `RelayIngestSink` proves only `publish` is reached — there is no
//      registrar on this seam).
//   3. The relayed packet is gated by `relay_world_uuid`: routed at B it is
//      delivered to a local recipient in the SAME relay world and NOT delivered
//      to one in a DIFFERENT relay world.
//
// No real QUIC sockets are used. The only relay pieces that genuinely require
// live sockets — the s2n-quic dial/accept in `PeerDialer` and the acceptor-side
// peer-cert routing — are exercised up to their boundary (the in-memory
// `RelayIngestSink` / outbound-queue seams the transport drains) and documented
// in the task report.

use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
use std::sync::Arc;
use std::time::Instant;

use common::game_data::Dimension;
use common::players::MinecraftPlayer;
use common::structs::packet::{
    AudioFramePacket, PacketOwner, PacketType, QuicNetworkPacket, QuicNetworkPacketData,
};
use common::structs::relay::RelayEndpoint;
use common::{Coordinate, Orientation, PlayerEnum};
use moka::future::Cache;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::stream::quic::connection_registry::{ConnectionRegistry, RoutedPacket};

use super::ingest_sink::RelayIngestSink;
use super::peer_manager::PeerManager;
use super::peer_table::PeerTable;
use super::presence::PresenceProver;
use super::relayed_packet::RelayedPacket;

const WORLD: &str = "W-shared-realm";
const OTHER_WORLD: &str = "W-other-realm";

fn ep(host: &str, port: u16) -> RelayEndpoint {
    RelayEndpoint {
        host: host.into(),
        port,
        primary: false,
    }
}

// Spy ingest sink: counts publishes (proving relayed packets reach the broadcast
// entry point) and captures the last packet. There is deliberately NO registrar
// here — the bypass is structural.
struct SpyIngest {
    count: AtomicUsize,
    last: std::sync::Mutex<Option<QuicNetworkPacket>>,
}

impl SpyIngest {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            count: AtomicUsize::new(0),
            last: std::sync::Mutex::new(None),
        })
    }
}

#[async_trait::async_trait]
impl RelayIngestSink for SpyIngest {
    async fn publish(&self, packet: QuicNetworkPacket) {
        self.count.fetch_add(1, AtomicOrdering::SeqCst);
        *self.last.lock().unwrap() = Some(packet);
    }
}

fn mc(name: &str, relay_world: &str, x: f32) -> PlayerEnum {
    PlayerEnum::Minecraft(MinecraftPlayer {
        name: name.into(),
        coordinates: Coordinate { x, y: 0.0, z: 0.0 },
        orientation: Orientation { x: 0.0, y: 0.0 },
        dimension: Dimension::Overworld,
        deafen: false,
        spectator: false,
        world_uuid: None,
        alternative_identity: None,
        player_uuid: None,
        relay_world_uuid: Some(relay_world.into()),
    })
}

// One relay setup (a single server's relay plane): peer table, prover, manager.
struct Node {
    endpoint: RelayEndpoint,
    peer_manager: Arc<PeerManager>,
    prover: Arc<PresenceProver>,
    ingest: Arc<SpyIngest>,
}

fn node(host: &str, port: u16, peer: &RelayEndpoint) -> Node {
    let endpoint = ep(host, port);
    let table = PeerTable::new_shared();
    table.set_active_worlds(vec![WORLD.to_string()]);
    table.set_world_peers(WORLD, vec![peer.clone()]);
    let prover = PresenceProver::new_shared();
    let ingest = SpyIngest::new();
    let peer_manager = PeerManager::new_shared(
        endpoint.clone(),
        table,
        ingest.clone(),
        prover.clone(),
    );
    peer_manager.set_prover(prover.clone());
    Node {
        endpoint,
        peer_manager,
        prover,
        ingest,
    }
}

#[tokio::test]
async fn relay_e2e_mutual_handshake_then_gated_relay() {
    let a_ep = ep("alpha", 5000);
    let b_ep = ep("bravo", 5000);
    let a = node("alpha", 5000, &b_ep);
    let b = node("bravo", 5000, &a_ep);
    let now = Instant::now();

    let a_key = PeerManager::endpoint_key(&a.endpoint);
    let b_key = PeerManager::endpoint_key(&b.endpoint);

    // --- Mutual presence handshake -------------------------------------------
    // A challenges: generate a token A must inject into the realm via its own
    // client. The realm fans it out; B's local client observes it.
    let a_challenges = a.peer_manager.challenges_to_send(now);
    assert_eq!(a_challenges.len(), 1, "A should challenge its unproven peer");
    let token_a = a_challenges[0].1.token.clone();

    // B challenges symmetrically.
    let b_challenges = b.peer_manager.challenges_to_send(now);
    assert_eq!(b_challenges.len(), 1);
    let token_b = b_challenges[0].1.token.clone();

    // Realm fan-out (hand-delivered): B's local client observed A's token; A's
    // local client observed B's token. Each node first registers the token it
    // expects to observe from its peer's challenge (only a known token is
    // echoable), modelling the realm delivering a legitimate token.
    b.peer_manager.expect_observed(&token_a, WORLD, now);
    a.peer_manager.expect_observed(&token_b, WORLD, now);
    b.peer_manager.on_local_client_observed(&token_a, now);
    a.peer_manager.on_local_client_observed(&token_b, now);

    // Each node drains the (token, world) pairs its client observed and echoes
    // them to the peer over the link. We deliver each echo into the OTHER node's
    // `route_observed_from_peer` (the peer-link reader path) and record the
    // echoing node's own half of the mutual proof, world-scoped.
    for (token, world) in b.peer_manager.tokens_to_echo_to_peer() {
        // B echoes A's token back to A over the link.
        a.peer_manager.route_observed_from_peer(&b_key, &token, now);
        b.prover.record_echoed_to_peer(&a_key, &world);
    }
    for (token, world) in a.peer_manager.tokens_to_echo_to_peer() {
        // A echoes B's token back to B over the link.
        b.peer_manager.route_observed_from_peer(&a_key, &token, now);
        a.prover.record_echoed_to_peer(&b_key, &world);
    }

    // Both gates now authorize peering MUTUALLY (peer echoed our token AND we
    // echoed theirs). A single direction would not have sufficed.
    assert!(
        a.prover.is_mutually_proven(&b_ep, WORLD),
        "A must consider B mutually proven for W"
    );
    assert!(
        b.prover.is_mutually_proven(&a_ep, WORLD),
        "B must consider A mutually proven for W"
    );

    // No further challenges once the peer has proven us.
    assert!(a.peer_manager.challenges_to_send(now).is_empty());
    assert!(b.peer_manager.challenges_to_send(now).is_empty());

    // --- Establish the (now-authorized) link & relay an audio packet ---------
    // A reconciles: it should dial B (lexically lower endpoint initiates).
    let a_dials = a.peer_manager.reconcile(now);
    assert_eq!(a_dials, vec![b_key.clone()], "A initiates to B post-proof");

    // B accepts the inbound peer connection (in production: an inbound QUIC conn
    // presenting a peer-identity cert routed to register_inbound).
    b.peer_manager.register_inbound(&a_key, now);

    // A produces a LOCAL audio packet in world W and forwards it to peers.
    let local_audio = audio_packet("alice", WORLD, 0.0);
    let forwarded = a
        .peer_manager
        .forward_local(&RelayedPacket::local(local_audio.clone()), WORLD);
    assert_eq!(forwarded, 1, "A forwards one copy to its single peer B");

    // The transport would deserialize that datagram on B's side and hand it to
    // B's ingest (FromPeer). We do that hop in-process here.
    b.peer_manager.ingest(&a_key, local_audio.clone()).await;
    assert_eq!(
        b.ingest.count.load(AtomicOrdering::SeqCst),
        1,
        "relayed packet must reach B's broadcast ingest (registration bypassed)"
    );

    // --- Gating by relay_world_uuid at B's broadcast fan-out -----------------
    // B has two local clients: bob (same relay world W) and carol (other world).
    // Routing the relayed packet must deliver to bob and NOT to carol.
    let registry = ConnectionRegistry::new();
    let cache: Arc<Cache<String, PlayerEnum>> = Arc::new(
        Cache::builder()
            .time_to_live(Duration::from_secs(60))
            .max_capacity(64)
            .build(),
    );
    cache.insert("bob".into(), mc("bob", WORLD, 0.0)).await;
    cache
        .insert("carol".into(), mc("carol", OTHER_WORLD, 0.0))
        .await;

    let (bob_tx, mut bob_rx) = mpsc::channel::<RoutedPacket>(8);
    let (carol_tx, mut carol_rx) = mpsc::channel::<RoutedPacket>(8);
    registry.register(b"bob".to_vec(), "bob".into(), bob_tx);
    registry.register(b"carol".to_vec(), "carol".into(), carol_tx);

    // The relayed packet, as ingested at B, carries A's sender (relay world W).
    let relayed_at_b = b.ingest.last.lock().unwrap().clone().unwrap();
    registry
        .route_audio_frame(&relayed_at_b, &cache, 16.0, 0.0)
        .await;

    assert!(
        bob_rx.try_recv().is_ok(),
        "recipient in the SAME relay world must receive the relayed audio"
    );
    assert!(
        carol_rx.try_recv().is_err(),
        "recipient in a DIFFERENT relay world must NOT receive it (relay_world_uuid gate)"
    );
}

fn audio_packet(owner: &str, relay_world: &str, x: f32) -> QuicNetworkPacket {
    QuicNetworkPacket {
        packet_type: PacketType::AudioFrame,
        owner: Some(PacketOwner {
            name: owner.into(),
            client_id: vec![1, 2, 3],
        }),
        data: QuicNetworkPacketData::AudioFrame(AudioFramePacket::new(
            vec![7, 7, 7],
            48000,
            Some(mc(owner, relay_world, x)),
            Some(true),
        )),
    }
}
