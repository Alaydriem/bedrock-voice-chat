use std::time::Duration;

use bvc_relay::node::NodeIdentity;
use bvc_relay::peer::{PeerEndpoint, PeerLink};
use bvc_server_lib::relay::PeerLinks;
use common::game_data::Dimension;
use common::structs::relay::wire::WireVersion;
use common::structs::relay::wire::datagram::{GroupScope, VoiceFrame};
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

fn frame() -> VoiceFrame {
    VoiceFrame {
        speaker: PlayerEnum::Minecraft(MinecraftPlayer {
            name: "Alice".to_string(),
            coordinates: Coordinate {
                x: 0.0,
                y: 0.0,
                z: 0.0,
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
        opus: vec![9],
        timestamp_ms: 0,
        spatial: true,
        jukebox: None,
    }
}

// An acceptor that hands back the first link it establishes, so the test can read
// from it. The link is returned rather than dropped: dropping a connection closes
// it and discards anything still in flight.
async fn accept_link(
    acceptor: &PeerEndpoint,
    worlds: Vec<String>,
) -> tokio::task::JoinHandle<PeerLink> {
    accept_link_at(acceptor, worlds, WireVersion(1)).await
}

async fn accept_link_at(
    acceptor: &PeerEndpoint,
    worlds: Vec<String>,
    version: WireVersion,
) -> tokio::task::JoinHandle<PeerLink> {
    let listening = acceptor.endpoint().clone();
    tokio::spawn(async move {
        let incoming = listening.accept().await.expect("incoming");
        let conn = incoming.await.expect("connection");
        PeerLink::establish(conn, worlds, version).expect("establish")
    })
}

async fn dial_link(
    dialer: &PeerEndpoint,
    acceptor: &PeerEndpoint,
    worlds: Vec<String>,
) -> PeerLink {
    dial_link_at(dialer, acceptor, worlds, WireVersion(1)).await
}

async fn dial_link_at(
    dialer: &PeerEndpoint,
    acceptor: &PeerEndpoint,
    worlds: Vec<String>,
    version: WireVersion,
) -> PeerLink {
    let conn = dialer
        .endpoint()
        .connect(loopback_addr(acceptor), PeerEndpoint::ALPN)
        .await
        .expect("dial");
    PeerLink::establish(conn, worlds, version).expect("establish")
}

// A version 1 peer cannot read the marker and would route members-only speech by proximity,
// so it is not sent the frame at all. A version 2 peer is.
#[tokio::test]
async fn broadcast_skips_members_only_on_v1_links() {
    let a_dir = TempDir::new().expect("tempdir");
    let b_dir = TempDir::new().expect("tempdir");
    let d_dir = TempDir::new().expect("tempdir");
    let old_peer = endpoint(&a_dir).await;
    let new_peer = endpoint(&b_dir).await;
    let dialer = endpoint(&d_dir).await;

    let old_task = accept_link_at(&old_peer, vec!["W1".to_string()], WireVersion(1)).await;
    let to_old = dial_link_at(&dialer, &old_peer, vec!["W1".to_string()], WireVersion(1)).await;
    let new_task =
        accept_link_at(&new_peer, vec!["W1".to_string()], WireVersion::GROUP_SCOPE).await;
    let to_new = dial_link_at(
        &dialer,
        &new_peer,
        vec!["W1".to_string()],
        WireVersion::GROUP_SCOPE,
    )
    .await;
    let _old_link = old_task.await.expect("join old");
    let _new_link = new_task.await.expect("join new");

    let links = PeerLinks::new();
    links.insert(to_old);
    links.insert(to_new);
    let group = GroupScope {
        channel: "ABCD-1234".to_string(),
        members_only: true,
    };

    assert_eq!(links.broadcast_world("W1", &frame(), Some(&group)), 1);
}

#[tokio::test]
async fn a_broadcast_reaches_only_the_links_carrying_that_world() {
    let a_dir = TempDir::new().expect("tempdir");
    let b_dir = TempDir::new().expect("tempdir");
    let d_dir = TempDir::new().expect("tempdir");
    let peer_a = endpoint(&a_dir).await;
    let peer_b = endpoint(&b_dir).await;
    let dialer = endpoint(&d_dir).await;

    let a_task = accept_link(&peer_a, vec!["W1".to_string()]).await;
    let to_a = dial_link(&dialer, &peer_a, vec!["W1".to_string()]).await;
    let b_task = accept_link(&peer_b, vec!["W2".to_string()]).await;
    let to_b = dial_link(&dialer, &peer_b, vec!["W2".to_string()]).await;

    let a_link = a_task.await.expect("join a");
    let b_link = b_task.await.expect("join b");

    let links = PeerLinks::new();
    links.insert(to_a);
    links.insert(to_b);

    assert_eq!(links.broadcast_world("W1", &frame(), None),1);

    let received = tokio::time::timeout(Duration::from_secs(5), a_link.recv())
        .await
        .expect("W1 peer must receive within the timeout")
        .expect("receive");
    assert_eq!(received.frame.opus, vec![9]);

    // The W2 peer must not have been sent anything at all.
    assert!(
        tokio::time::timeout(Duration::from_millis(300), b_link.recv())
            .await
            .is_err(),
        "a peer that does not carry the world must receive nothing"
    );
}

#[tokio::test]
async fn a_broadcast_for_an_uncarried_world_reaches_nobody() {
    let a_dir = TempDir::new().expect("tempdir");
    let d_dir = TempDir::new().expect("tempdir");
    let peer_a = endpoint(&a_dir).await;
    let dialer = endpoint(&d_dir).await;

    let a_task = accept_link(&peer_a, vec!["W1".to_string()]).await;
    let to_a = dial_link(&dialer, &peer_a, vec!["W1".to_string()]).await;
    let _a_link = a_task.await.expect("join a");

    let links = PeerLinks::new();
    links.insert(to_a);

    assert_eq!(links.broadcast_world("W-other", &frame(), None),0);
}

#[tokio::test]
async fn a_removed_link_stops_receiving() {
    let a_dir = TempDir::new().expect("tempdir");
    let d_dir = TempDir::new().expect("tempdir");
    let peer_a = endpoint(&a_dir).await;
    let dialer = endpoint(&d_dir).await;

    let a_task = accept_link(&peer_a, vec!["W1".to_string()]).await;
    let to_a = dial_link(&dialer, &peer_a, vec!["W1".to_string()]).await;
    let _a_link = a_task.await.expect("join a");

    let node = to_a.node();
    let links = PeerLinks::new();
    links.insert(to_a);
    assert_eq!(links.broadcast_world("W1", &frame(), None),1);

    links.remove(&node);

    assert!(links.is_empty());
    assert_eq!(links.broadcast_world("W1", &frame(), None),0);
}

// Two bridges fronting one world is legitimate, so this reports rather than
// refuses — but it is also what a misconfigured second bridge looks like, and
// it doubles every frame in that world.
#[tokio::test]
async fn a_world_already_carried_is_reported() {
    let a_dir = TempDir::new().expect("tempdir");
    let d_dir = TempDir::new().expect("tempdir");
    let peer_a = endpoint(&a_dir).await;
    let dialer = endpoint(&d_dir).await;

    let a_task = accept_link(&peer_a, vec!["W1".to_string()]).await;
    let first = dial_link(&dialer, &peer_a, vec!["W1".to_string()]).await;
    let _a_link = a_task.await.expect("join a");

    let b_dir = TempDir::new().expect("tempdir");
    let peer_b = endpoint(&b_dir).await;
    let b_task = accept_link(&peer_b, vec!["W1".to_string(), "W2".to_string()]).await;
    let second = dial_link(&dialer, &peer_b, vec!["W1".to_string(), "W2".to_string()]).await;
    let _b_link = b_task.await.expect("join b");

    let links = PeerLinks::new();
    links.insert(first);

    assert_eq!(
        links.worlds_also_carried(&second),
        vec!["W1".to_string()],
        "only the overlapping world is reported"
    );
}

#[tokio::test]
async fn a_link_carrying_only_new_worlds_reports_nothing() {
    let a_dir = TempDir::new().expect("tempdir");
    let d_dir = TempDir::new().expect("tempdir");
    let peer_a = endpoint(&a_dir).await;
    let dialer = endpoint(&d_dir).await;

    let a_task = accept_link(&peer_a, vec!["W1".to_string()]).await;
    let only = dial_link(&dialer, &peer_a, vec!["W1".to_string()]).await;
    let _a_link = a_task.await.expect("join a");

    let links = PeerLinks::new();

    assert!(links.worlds_also_carried(&only).is_empty());
}
