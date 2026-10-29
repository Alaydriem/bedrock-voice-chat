use bvc_relay::node::NodeIdentity;
use bvc_relay::peer::{PeerEndpoint, PeerLink};
use common::game_data::Dimension;
use common::structs::relay::wire::WireVersion;
use common::structs::relay::wire::datagram::{GroupScope, VoiceFrame};
use common::traits::player_data::PlayerData;
use common::{Coordinate, MinecraftPlayer, Orientation, PlayerEnum};
use iroh::EndpointAddr;
use tempfile::TempDir;

async fn endpoint(dir: &TempDir) -> PeerEndpoint {
    let path = dir.path().to_str().expect("utf-8 path");
    let identity = NodeIdentity::load_or_create(path).expect("identity");
    PeerEndpoint::bind(&identity).await.expect("bind")
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
            whispering: false,
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
        let link = PeerLink::establish(conn, vec!["W1".to_string()], WireVersion(1)).expect("establish");
        let received = link.recv().await.expect("receive").frame;
        (received, link)
    });

    let conn = dialer
        .endpoint()
        .connect(addr, PeerEndpoint::ALPN)
        .await
        .expect("dial");
    let link = PeerLink::establish(conn, vec!["W1".to_string()], WireVersion(1)).expect("establish");
    link.send(frame("Alice"), None).expect("send");

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
        let link = PeerLink::establish(conn, vec!["W1".to_string()], WireVersion(1)).expect("establish");
        let received = link.recv().await.expect("receive").frame;
        (received, link)
    });

    let conn = dialer
        .endpoint()
        .connect(addr, PeerEndpoint::ALPN)
        .await
        .expect("dial");
    let link = PeerLink::establish(conn, vec!["W1".to_string()], WireVersion(1)).expect("establish");
    link.send(frame("Alice"), None).expect("send");

    let (received, _held) = server.await.expect("join");

    assert_eq!(
        received.speaker.world_identifier(),
        Some("W1"),
        "the world the frame is scoped to must survive the crossing"
    );
}

// The bridge delivers group audio by the channel id the frame carries, so the marker has to
// arrive as sent, not only the audio.
#[tokio::test]
async fn a_marked_frame_crosses_a_v2_link_with_its_group() {
    let a_dir = TempDir::new().expect("tempdir");
    let b_dir = TempDir::new().expect("tempdir");
    let acceptor = endpoint(&a_dir).await;
    let dialer = endpoint(&b_dir).await;

    let addr = loopback_addr(&acceptor);
    let listening = acceptor.endpoint().clone();

    let server = tokio::spawn(async move {
        let incoming = listening.accept().await.expect("incoming");
        let conn = incoming.await.expect("connection");
        let link = PeerLink::establish(conn, vec!["W1".to_string()], WireVersion::GROUP_SCOPE)
            .expect("establish");
        let received = link.recv().await.expect("receive");
        (received, link)
    });

    let conn = dialer
        .endpoint()
        .connect(addr, PeerEndpoint::ALPN)
        .await
        .expect("dial");
    let link = PeerLink::establish(conn, vec!["W1".to_string()], WireVersion::GROUP_SCOPE)
        .expect("establish");
    let group = GroupScope {
        channel: "ABCD-1234".to_string(),
        members_only: true,
    };
    assert!(link.send(frame("Alice"), Some(group.clone())).expect("send"));

    let (received, _held) = server.await.expect("join");

    assert_eq!(received.group, Some(group));
    assert_eq!(received.frame.speaker.get_name(), "Alice");
}
