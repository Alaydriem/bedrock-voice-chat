use std::sync::Arc;
use std::time::Duration;

use bvc_relay::node::{NodeIdentity, PeerTicket};
use bvc_relay::peer::session::{PeerSession, SessionConfig};
use bvc_relay::peer::{Handshake, PeerAuthority, PeerEndpoint, PeerLink, PeerScope};
use common::game_data::Dimension;
use common::structs::relay::Capability;
use common::structs::relay::wire::datagram::VoiceFrame;
use common::{Coordinate, MinecraftPlayer, Orientation, PlayerEnum};
use tempfile::TempDir;

struct AcceptsAnyone;

impl PeerAuthority for AcceptsAnyone {
    fn authorize(&self, _node: &iroh::PublicKey, declared: &[String]) -> Option<PeerScope> {
        Some(PeerScope {
            worlds: declared.to_vec(),
            capabilities: vec![Capability::CarrySpeakers],
        })
    }
}

fn frame(marker: u8) -> VoiceFrame {
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
            deafen: false,
            spectator: false,
            world_uuid: None,
            alternative_identity: None,
            player_uuid: None,
            relay_world_uuid: Some("W1".to_string()),
        }),
        sample_rate: 48000,
        opus: vec![marker],
        timestamp_ms: 0,
        spatial: true,
        jukebox: None,
    }
}

// Binds an acceptor that takes anyone and sends `burst` frames down every link
// it accepts, then holds the link open and silent.
//
// Bounded rather than continuous so a reader can actually park: against a peer
// that never stops talking, a test asserting that a parked read is released
// would be asserting which of the two won a race.
//
// The endpoint is returned so the caller keeps it alive — dropping it closes the
// socket the session is dialling.
async fn acceptor(dir: &TempDir, burst: usize) -> (String, Arc<PeerEndpoint>) {
    let identity =
        NodeIdentity::load_or_create(dir.path().to_str().expect("path")).expect("identity");
    let endpoint = Arc::new(
        PeerEndpoint::bind(&identity, None)
            .await
            .expect("bind acceptor"),
    );
    let ticket = endpoint.ticket().await.expect("mint");

    let listening = Arc::clone(&endpoint);
    tokio::spawn(async move {
        while let Some(incoming) = listening.endpoint().accept().await {
            tokio::spawn(async move {
                let Ok(conn) = incoming.await else { return };
                let Ok(accept) = Handshake::accept(&conn, &AcceptsAnyone).await else {
                    return;
                };
                let Ok(link) = PeerLink::establish(conn, accept.worlds) else {
                    return;
                };

                let mut ticker = tokio::time::interval(Duration::from_millis(50));
                for counter in 0..burst {
                    ticker.tick().await;
                    if link.send(frame(counter as u8 + 1)).is_err() {
                        return;
                    }
                }

                // Held open and quiet. Dropping the link here would close the
                // connection, which the session would treat as a disconnect and
                // redial.
                while link.recv().await.is_ok() {}
            });
        }
    });

    (ticket, endpoint)
}

fn config(node_dir: &TempDir, peerlink: String) -> SessionConfig {
    SessionConfig {
        node_dir: node_dir.path().to_str().expect("path").to_string(),
        peerlink,
        worlds: vec!["W1".to_string()],
        relay_url: None,
        inbox_capacity: 8,
    }
}

#[tokio::test]
async fn a_session_delivers_a_frame_from_the_peer() {
    let acceptor_dir = TempDir::new().expect("tempdir");
    let dialer_dir = TempDir::new().expect("tempdir");
    let (ticket, _acceptor) = acceptor(&acceptor_dir, 4).await;

    let session = PeerSession::open(config(&dialer_dir, ticket))
        .await
        .expect("open");

    let received = tokio::time::timeout(Duration::from_secs(15), session.next())
        .await
        .expect("a frame must arrive within the timeout")
        .expect("a frame");

    assert!(!received.opus.is_empty());
    session.close().await;
}

// The constraint the whole SDK surface is shaped around: uniffi cannot cancel a
// parked call, so a session that cannot be ended from outside cannot be shut
// down at all.
#[tokio::test]
async fn close_ends_a_parked_read() {
    let acceptor_dir = TempDir::new().expect("tempdir");
    let dialer_dir = TempDir::new().expect("tempdir");
    let (ticket, _acceptor) = acceptor(&acceptor_dir, 2).await;

    let session = PeerSession::open(config(&dialer_dir, ticket))
        .await
        .expect("open");

    // Drain until the peer goes quiet, so the reader below is genuinely parked
    // rather than about to be handed a queued frame.
    while tokio::time::timeout(Duration::from_secs(2), session.next())
        .await
        .is_ok()
    {}

    let reader = tokio::spawn({
        let session = Arc::clone(&session);
        async move { session.next().await }
    });

    tokio::time::sleep(Duration::from_millis(200)).await;
    session.close().await;

    let parked = tokio::time::timeout(Duration::from_secs(5), reader)
        .await
        .expect("close must release a parked read")
        .expect("join");

    assert!(parked.is_none());
}

// A bridge prints its own link for the operator to paste into config.hcl, so the
// session has to produce one for the identity it is actually using rather than
// echo the one it was given.
#[tokio::test]
async fn a_session_reports_its_own_peer_link() {
    let acceptor_dir = TempDir::new().expect("tempdir");
    let dialer_dir = TempDir::new().expect("tempdir");
    let (ticket, _acceptor) = acceptor(&acceptor_dir, 1).await;

    let session = PeerSession::open(config(&dialer_dir, ticket.clone()))
        .await
        .expect("open");

    let own = session.peerlink().await.expect("mint");
    assert!(own.starts_with("bvcpeer"), "not a peer link: {own}");
    assert_ne!(own, ticket, "reported the peer's link, not its own");

    session.close().await;
}

// An unreachable peer is the ordinary startup order — a plugin loads before the
// server it talks to. Open must succeed and keep retrying rather than failing the
// plugin's enable.
#[tokio::test]
async fn open_succeeds_against_an_unreachable_peer() {
    let unreachable_dir = TempDir::new().expect("tempdir");
    let dialer_dir = TempDir::new().expect("tempdir");

    let identity = NodeIdentity::load_or_create(unreachable_dir.path().to_str().expect("path"))
        .expect("identity");
    let ticket =
        PeerTicket::mint(&iroh::EndpointAddr::new(identity.node_id())).expect("mint a ticket");

    let session = PeerSession::open(config(&dialer_dir, ticket))
        .await
        .expect("open must not depend on the peer being up");

    assert!(!session.is_connected());
    session.close().await;
}
