use std::time::Duration;

use bvc_relay::node::{NodeIdentity, PeerTicket};
use bvc_relay::peer::{Handshake, PeerAuthority, PeerEndpoint, PeerLink, PeerScope};
use common::game_data::Dimension;
use common::structs::relay::Capability;
use common::structs::relay::wire::datagram::VoiceFrame;
use common::{Coordinate, MinecraftPlayer, Orientation, PlayerEnum};
use iroh::endpoint::IncomingAddr;
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
            deafen: false,
            spectator: false,
            world_uuid: None,
            alternative_identity: None,
            player_uuid: None,
            relay_world_uuid: Some("W1".to_string()),
            bridged_voice: false,
        }),
        sample_rate: 48000,
        opus: vec![3, 1, 4],
        timestamp_ms: 0,
        spatial: true,
        jukebox: None,
    }
}

// A bridge sharing a host with BVC has no public address and no relay. The
// ticket is the only thing it is given, so the ticket has to be enough on its
// own — which is true only because it carries direct addresses.
#[tokio::test]
async fn a_ticket_reaches_a_peer_on_the_same_host_without_a_relay() {
    let a_dir = TempDir::new().expect("tempdir");
    let b_dir = TempDir::new().expect("tempdir");

    let server_identity =
        NodeIdentity::load_or_create(a_dir.path().to_str().expect("path")).expect("identity");
    let bridge_identity =
        NodeIdentity::load_or_create(b_dir.path().to_str().expect("path")).expect("identity");

    // No relay on either side: this is the whole point of the case.
    let server = PeerEndpoint::bind(&server_identity, None)
        .await
        .expect("bind server");
    let bridge = PeerEndpoint::bind(&bridge_identity, None)
        .await
        .expect("bind bridge");

    let ticket = server.ticket().await.expect("mint a ticket");
    let addr = PeerTicket::parse(&ticket).expect("parse");

    assert!(
        addr.ip_addrs().any(|socket| socket.ip().is_loopback()),
        "a ticket minted with no relay must carry a loopback address, or a \
         bridge on an isolated host has nothing to dial: {addr:?}"
    );

    // Every address the ticket offers belongs to this host, which is what makes
    // any direct path taken from it a local one.
    let advertised: Vec<std::net::IpAddr> = addr.ip_addrs().map(|socket| socket.ip()).collect();

    let listening = server.endpoint().clone();
    let accepted = tokio::spawn(async move {
        let incoming = listening.accept().await.expect("incoming");
        // Read before awaiting: this is how the connection actually arrived,
        // rather than how the ticket said it might.
        let arrived_at = incoming.remote_addr();
        let conn = incoming.await.expect("connection");
        let accept = Handshake::accept(&conn, &AcceptsAnyone)
            .await
            .expect("accept");
        let link = PeerLink::establish(conn, accept.worlds).expect("establish");
        (arrived_at, link.recv().await.expect("receive"))
    });

    let conn = bridge
        .endpoint()
        .connect(addr, PeerEndpoint::ALPN)
        .await
        .expect("dial the ticket");
    let accept = Handshake::dial(&conn, vec!["W1".to_string()])
        .await
        .expect("handshake");
    let link = PeerLink::establish(conn, accept.worlds).expect("establish");

    link.send(frame()).expect("send");

    let (arrived_at, received) = tokio::time::timeout(Duration::from_secs(5), accepted)
        .await
        .expect("the same-host peer must receive within the timeout")
        .expect("join");

    assert_eq!(received.opus, vec![3, 1, 4]);

    // The requirement is that a same-host peer never leaves the machine.
    //
    // A direct IP path is asserted rather than a loopback one specifically. Iroh
    // probes every candidate in the ticket in parallel and keeps whichever
    // answers first, with no preference for loopback, so on a host with a live
    // interface it usually settles on that interface's own address. Traffic to a
    // local address is still handled by the kernel locally and never reaches the
    // wire, and every address in this ticket belongs to this host — so a direct
    // path here is a local one whichever of them won.
    //
    // What would break the requirement is a relayed path, and that is what this
    // rules out. Both endpoints were bound with no relay, so it is impossible by
    // construction as well as unobserved.
    match arrived_at {
        IncomingAddr::Ip(socket) => assert!(
            socket.ip().is_loopback() || advertised.contains(&socket.ip()),
            "same-host peering arrived at {socket}, which is not an address this \
             host advertised — the traffic did not stay local"
        ),
        other => panic!("same-host peering must not use a relay, got {other:?}"),
    }
}
