use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use bvc_relay::node::NodeIdentity;
use bvc_relay::peer::PeerEndpoint;
use bvc_relay::node::PeerTicket;
use bvc_server_lib::config::PeerConfig;
use bvc_server_lib::relay::{GrantTable, LocalClients, PeerPlane, PeerSink};
use common::game_data::Dimension;
use common::structs::packet::{
    AudioFramePacket, PacketType, QuicNetworkPacket, QuicNetworkPacketData,
};
use common::traits::player_data::PlayerData;
use common::{Coordinate, MinecraftPlayer, Orientation, PlayerEnum};
use common::structs::packet::SpeakerPosition;
use iroh::EndpointAddr;
use tempfile::TempDir;
use tokio::sync::mpsc;

struct NoLocals;

impl LocalClients for NoLocals {
    fn has_live_client(&self, _identity: &str) -> bool {
        false
    }
}

// A sink that makes arrival observable.
struct ChannelSink(mpsc::UnboundedSender<QuicNetworkPacket>);

impl PeerSink for ChannelSink {
    fn publish(&self, packet: QuicNetworkPacket) {
        let _ = self.0.send(packet);
    }
}

fn speaker(world: Option<&str>) -> PlayerEnum {
    PlayerEnum::Minecraft(MinecraftPlayer {
        name: "Alice".to_string(),
        coordinates: Coordinate {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        },
        orientation: Orientation { x: 0.0, y: 0.0 },
        dimension: Dimension::Overworld,
        deafen: false,
        spectator: false,
        world_uuid: None,
        alternative_identity: None,
        player_uuid: None,
        relay_world_uuid: world.map(String::from),
        bridged_voice: false,
    })
}

fn local_audio(world: Option<&str>) -> QuicNetworkPacket {
    QuicNetworkPacket {
        packet_type: PacketType::AudioFrame,
        data: QuicNetworkPacketData::AudioFrame(AudioFramePacket::new(
            vec![7, 7, 7],
            Some(SpeakerPosition::from_player(&speaker(world))),
            Some(true),
        )),
        ..Default::default()
    }
}

fn grants_for(node: iroh::PublicKey, worlds: &[&str]) -> Arc<GrantTable> {
    let mut map = HashMap::new();
    map.insert(
        "peer".to_string(),
        PeerConfig {
            peerlink: PeerTicket::mint(&EndpointAddr::new(node)).expect("mint"),
            worlds: worlds.iter().map(|w| w.to_string()).collect(),
            capabilities: vec!["carry_speakers".to_string()],
        },
    );
    Arc::new(GrantTable::from_config(&map).expect("valid config"))
}

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

struct Pair {
    dialer: Arc<PeerPlane>,
    acceptor_rx: mpsc::UnboundedReceiver<QuicNetworkPacket>,
    _acceptor: Arc<PeerPlane>,
    _dirs: (TempDir, TempDir),
}

// Two planes that declare each other for `world`, with the dialer already
// connected to the acceptor.
async fn peered(world: &str) -> Pair {
    let a_dir = TempDir::new().expect("tempdir");
    let b_dir = TempDir::new().expect("tempdir");

    let acceptor_identity =
        NodeIdentity::load_or_create(a_dir.path().to_str().expect("path")).expect("identity");
    let dialer_identity =
        NodeIdentity::load_or_create(b_dir.path().to_str().expect("path")).expect("identity");

    let (tx, acceptor_rx) = mpsc::unbounded_channel();
    let (dead_tx, _dead_rx) = mpsc::unbounded_channel();

    let acceptor = PeerPlane::bind(
        &acceptor_identity,
        grants_for(dialer_identity.node_id(), &[world]),
        Arc::new(NoLocals),
        Arc::new(ChannelSink(tx)),
        Arc::new(moka::future::Cache::new(16)),
        None,
        None,
    )
    .await
    .expect("bind acceptor");
    acceptor.spawn_accept_loop();

    let dialer = PeerPlane::bind(
        &dialer_identity,
        grants_for(acceptor_identity.node_id(), &[world]),
        Arc::new(NoLocals),
        Arc::new(ChannelSink(dead_tx)),
        Arc::new(moka::future::Cache::new(16)),
        None,
        None,
    )
    .await
    .expect("bind dialer");

    dialer
        .dial(loopback_addr(acceptor.endpoint()), vec![world.to_string()])
        .await
        .expect("dial succeeds");

    Pair {
        dialer,
        acceptor_rx,
        _acceptor: acceptor,
        _dirs: (a_dir, b_dir),
    }
}

#[tokio::test]
async fn local_audio_forwarded_by_one_plane_arrives_admitted_at_the_other() {
    let mut pair = peered("W1").await;

    assert_eq!(
        pair.dialer.forward_local(&local_audio(Some("W1")), &speaker(Some("W1"))),
        1,
        "the granted world must reach exactly one peer"
    );

    let packet = tokio::time::timeout(Duration::from_secs(5), pair.acceptor_rx.recv())
        .await
        .expect("the peer must publish within the timeout")
        .expect("a packet");

    assert_eq!(packet.packet_type, PacketType::AudioFrame);
    let QuicNetworkPacketData::AudioFrame(audio) = &packet.data else {
        panic!("expected an audio frame");
    };
    assert_eq!(audio.data, vec![7, 7, 7]);
    assert!(
        audio.speaker.is_some(),
        "the speaker's position must survive the crossing"
    );
    assert_eq!(
        packet.sender_identity().map(|i| i.to_string()).as_deref(),
        Some("minecraft:Alice"),
        "and the receiving server mints the envelope sender that names them"
    );
}

#[tokio::test]
async fn audio_for_an_ungranted_world_is_not_forwarded() {
    let mut pair = peered("W1").await;

    assert_eq!(
        pair.dialer.forward_local(&local_audio(Some("W-other")), &speaker(Some("W-other"))),
        0,
        "no link carries that world"
    );

    assert!(
        tokio::time::timeout(Duration::from_millis(300), pair.acceptor_rx.recv())
            .await
            .is_err(),
        "nothing must reach the peer"
    );
}

#[tokio::test]
async fn audio_with_no_relay_world_is_not_forwarded() {
    let mut pair = peered("W1").await;

    assert_eq!(pair.dialer.forward_local(&local_audio(None), &speaker(None)), 0);

    assert!(
        tokio::time::timeout(Duration::from_millis(300), pair.acceptor_rx.recv())
            .await
            .is_err()
    );
}
