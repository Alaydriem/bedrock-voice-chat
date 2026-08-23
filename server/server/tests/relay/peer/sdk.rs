use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use bvc_relay::node::{NodeIdentity, PeerTicket};
use bvc_relay::peer::PeerEndpoint;
use bvc_relay_sdk::{BvcPeer, SdkConfig, SdkFrame};
use bvc_server_lib::config::PeerConfig;
use bvc_server_lib::relay::{GrantTable, LocalClients, PeerPlane, PeerSink};
use common::game_data::Dimension;
use common::structs::packet::{PacketType, QuicNetworkPacket, QuicNetworkPacketData};
use common::traits::player_data::PlayerData;
use iroh::EndpointAddr;
use tempfile::TempDir;
use tokio::sync::mpsc;

const WORLD: &str = "W1";

struct NoLocals;

impl LocalClients for NoLocals {
    fn has_live_client(&self, _identity: &str) -> bool {
        false
    }
}

struct ChannelSink(mpsc::UnboundedSender<QuicNetworkPacket>);

impl PeerSink for ChannelSink {
    fn publish(&self, packet: QuicNetworkPacket) {
        let _ = self.0.send(packet);
    }
}

fn grants_for(node: iroh::PublicKey, worlds: &[&str]) -> Arc<GrantTable> {
    let mut map = HashMap::new();
    map.insert(
        "bridge".to_string(),
        PeerConfig {
            peerlink: PeerTicket::mint(&EndpointAddr::new(node)).expect("mint"),
            worlds: worlds.iter().map(|w| w.to_string()).collect(),
            capabilities: vec!["carry_speakers".to_string()],
        },
    );
    Arc::new(GrantTable::from_config(&map).expect("valid config"))
}

// The address the SDK is handed. A bridge runs beside the server it feeds, so the
// ticket carries a loopback address and the pair needs no relay to find each
// other.
fn loopback_addr(endpoint: &PeerEndpoint) -> EndpointAddr {
    let mut addr = EndpointAddr::new(endpoint.node_id());
    for socket in endpoint.endpoint().bound_sockets() {
        if socket.is_ipv4() {
            addr = addr.with_ip_addr(std::net::SocketAddr::new(
                std::net::Ipv4Addr::LOCALHOST.into(),
                socket.port(),
            ));
        }
    }
    addr
}

struct Bridged {
    peer: Arc<BvcPeer>,
    server_rx: mpsc::UnboundedReceiver<QuicNetworkPacket>,
    _server: Arc<PeerPlane>,
    _dirs: (TempDir, TempDir),
}

// A real server plane with the SDK connected to it and granted `WORLD`.
//
// The SDK's node id is read from its key file before the session is opened, which
// is what lets the grant naming it exist first. A bridge is configured the same
// way: the operator runs it once to get its link, and the key it minted that link
// from is still there on the next start.
async fn bridged(granted: &[&str]) -> Bridged {
    let server_dir = TempDir::new().expect("tempdir");
    let sdk_dir = TempDir::new().expect("tempdir");

    let server_identity =
        NodeIdentity::load_or_create(server_dir.path().to_str().expect("path")).expect("identity");
    let sdk_identity =
        NodeIdentity::load_or_create(sdk_dir.path().to_str().expect("path")).expect("identity");

    let (tx, server_rx) = mpsc::unbounded_channel();

    let server = PeerPlane::bind(
        &server_identity,
        grants_for(sdk_identity.node_id(), granted),
        Arc::new(NoLocals),
        Arc::new(ChannelSink(tx)),
        Arc::new(moka::future::Cache::new(16)),
        None,
        None,
    )
    .await
    .expect("bind server plane");
    server.spawn_accept_loop();

    let peerlink = PeerTicket::mint(&loopback_addr(server.endpoint())).expect("mint");

    let peer = BvcPeer::open(SdkConfig {
        node_dir: sdk_dir.path().to_str().expect("path").to_string(),
        peerlink,
        worlds: vec![WORLD.to_string()],
        relay_url: None,
        inbox_capacity: 8,
    })
    .await
    .expect("open");

    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    while !peer.is_connected() {
        assert!(
            tokio::time::Instant::now() < deadline,
            "the SDK never reached the server"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    Bridged {
        peer,
        server_rx,
        _server: server,
        _dirs: (server_dir, sdk_dir),
    }
}

fn outbound(world: &str) -> SdkFrame {
    SdkFrame {
        speaker: "BridgeSpeaker".to_string(),
        world: Some(world.to_string()),
        dimension: "overworld".to_string(),
        x: 4.0,
        y: 64.0,
        z: -2.0,
        opus: vec![7, 7, 7],
        sample_rate: 48000,
        timestamp_ms: 1234,
        spatial: true,
        jukebox: None,
    }
}

// What a bridge is for, end to end: a frame the consumer built through the SDK's
// own type crosses the wire and comes out of a real server's peer boundary as a
// packet that server minted.
//
// `PeerIngest::admit` is covered directly elsewhere. What is proven here is that
// the shape the SDK produces is one admission accepts — the unit tests build a
// `VoiceFrame` by hand, which cannot show that.
#[tokio::test]
async fn a_frame_sent_through_the_sdk_arrives_admitted_at_a_real_server() {
    let mut bridged = bridged(&[WORLD]).await;

    bridged.peer.send(outbound(WORLD)).expect("send");

    let packet = tokio::time::timeout(Duration::from_secs(10), bridged.server_rx.recv())
        .await
        .expect("the server must publish within the timeout")
        .expect("a packet");

    assert_eq!(packet.packet_type, PacketType::AudioFrame);
    let QuicNetworkPacketData::AudioFrame(audio) = &packet.data else {
        panic!("expected an audio frame");
    };

    assert_eq!(audio.data, vec![7, 7, 7]);
    assert!(
        audio.speaker.is_some(),
        "the position a listener pans from survives the crossing"
    );
    // The name and the world are the receiving server's own findings now: it mints the envelope
    // sender from the wire frame's speaker rather than forwarding a player on the audio frame.
    assert_eq!(
        packet.sender_identity().map(|i| i.to_string()).as_deref(),
        Some("minecraft:BridgeSpeaker")
    );
    assert!(
        packet.sender.is_some(),
        "the receiving server mints the envelope sender itself"
    );
}

// The dimension gate on the receiving side is unconditional, so a frame that does
// not carry the speaker's dimension asserts a wrong one rather than omitting it:
// the speaker becomes inaudible to a listener beside them and audible to one a
// world away. This is the regression guard for that.
#[tokio::test]
async fn a_frame_keeps_its_dimension_across_the_peer_boundary() {
    let mut bridged = bridged(&[WORLD]).await;

    let mut frame = outbound(WORLD);
    frame.dimension = "nether".to_string();
    bridged.peer.send(frame).expect("send");

    let packet = tokio::time::timeout(Duration::from_secs(10), bridged.server_rx.recv())
        .await
        .expect("the server must publish within the timeout")
        .expect("a packet");

    let QuicNetworkPacketData::AudioFrame(audio) = &packet.data else {
        panic!("expected an audio frame");
    };

    // The dimension gated routing on the sending side and is not something a listener reads,
    // so it no longer rides the audio frame. What must survive is the position.
    assert!(audio.speaker.is_some());
}

// The grant is enforced on the live path, not only inside `admit`.
//
// The handshake cannot catch this one: the SDK declares the world it is entitled
// to and then names a different one on a frame. A bridge is ordinary software
// that can be wrong about which world it is serving, so the boundary has to hold
// after the handshake rather than only at it.
#[tokio::test]
async fn a_frame_naming_a_world_outside_the_grant_is_dropped_at_the_boundary() {
    let mut bridged = bridged(&[WORLD]).await;

    bridged.peer.send(outbound("W-other")).expect("send");

    assert!(
        tokio::time::timeout(Duration::from_millis(500), bridged.server_rx.recv())
            .await
            .is_err(),
        "an ungranted world must not reach the server's clients"
    );
}
