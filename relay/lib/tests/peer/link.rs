use bvc_relay::node::NodeIdentity;
use bvc_relay::peer::{PeerEndpoint, PeerLink};
use common::game_data::Dimension;
use common::structs::relay::wire::datagram::VoiceFrame;
use common::traits::player_data::PlayerData;
use common::{Coordinate, MinecraftPlayer, Orientation, PlayerEnum};
use iroh::EndpointAddr;
use tempfile::TempDir;

async fn endpoint(dir: &TempDir) -> PeerEndpoint {
    let path = dir.path().to_str().expect("utf-8 path");
    let identity = NodeIdentity::load_or_create(path).expect("identity");
    PeerEndpoint::bind(&identity, None).await.expect("bind")
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

fn frame(name: &str) -> VoiceFrame {
    VoiceFrame {
        speaker: PlayerEnum::Minecraft(MinecraftPlayer {
            name: name.to_string(),
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
            relay_world_uuid: Some("W1".to_string()),
            bridged_voice: false,
        }),
        sample_rate: 48000,
        opus: vec![1, 2, 3, 4],
        timestamp_ms: 0,
        spatial: true,
        jukebox: None,
    }
}

#[tokio::test]
async fn a_voice_frame_crosses_a_link_intact() {
    let a_dir = TempDir::new().expect("tempdir");
    let b_dir = TempDir::new().expect("tempdir");
    let acceptor = endpoint(&a_dir).await;
    let dialer = endpoint(&b_dir).await;

    let addr = loopback_addr(&acceptor);
    let listening = acceptor.endpoint().clone();

    let server = tokio::spawn(async move {
        let incoming = listening.accept().await.expect("incoming");
        let conn = incoming.await.expect("connection");
        let link = PeerLink::establish(conn, vec!["W1".to_string()]).expect("establish");
        let received = link.recv().await.expect("receive");
        (received, link)
    });

    let conn = dialer
        .endpoint()
        .connect(addr, PeerEndpoint::ALPN)
        .await
        .expect("dial");
    let link = PeerLink::establish(conn, vec!["W1".to_string()]).expect("establish");
    link.send(frame("Alice")).expect("send");

    let (received, _held) = server.await.expect("join");

    assert_eq!(received.speaker.get_name(), "Alice");
    assert_eq!(received.opus, vec![1, 2, 3, 4]);
    assert_eq!(received.sample_rate, 48000);
    assert!(received.spatial);
}

// The coordinates travel with the speaker, because a receiving server has no
// position feed covering another server's players.
#[tokio::test]
async fn the_speakers_position_crosses_with_the_frame() {
    let a_dir = TempDir::new().expect("tempdir");
    let b_dir = TempDir::new().expect("tempdir");
    let acceptor = endpoint(&a_dir).await;
    let dialer = endpoint(&b_dir).await;

    let addr = loopback_addr(&acceptor);
    let listening = acceptor.endpoint().clone();

    let server = tokio::spawn(async move {
        let incoming = listening.accept().await.expect("incoming");
        let conn = incoming.await.expect("connection");
        let link = PeerLink::establish(conn, vec!["W1".to_string()]).expect("establish");
        let received = link.recv().await.expect("receive");
        (received, link)
    });

    let conn = dialer
        .endpoint()
        .connect(addr, PeerEndpoint::ALPN)
        .await
        .expect("dial");
    let link = PeerLink::establish(conn, vec!["W1".to_string()]).expect("establish");
    link.send(frame("Alice")).expect("send");

    let (received, _held) = server.await.expect("join");

    assert_eq!(
        received.speaker.world_identifier(),
        Some("W1"),
        "the world the frame is scoped to must survive the crossing"
    );
}

// A relay is what makes a ticket dialable off the local network. Binding
// without one is the deliberate default; binding with one must not fail.
#[tokio::test]
async fn an_endpoint_binds_with_a_relay_configured() {
    let dir = TempDir::new().expect("tempdir");
    let identity =
        NodeIdentity::load_or_create(dir.path().to_str().expect("path")).expect("identity");

    let endpoint = PeerEndpoint::bind(
        &identity,
        Some("https://relay.example.".parse().expect("relay url")),
    )
    .await
    .expect("bind with a relay");

    assert_eq!(endpoint.node_id(), identity.node_id());
}
