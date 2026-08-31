use std::time::Duration;

use bvc_relay::node::{NodeIdentity, PeerTicket};
use bvc_relay::peer::{AddressObserver, PeerEndpoint};
use tempfile::TempDir;

// The whole exchange over a real connection: a node dials, and is told the address
// it was seen at. This is the only way a server behind NAT can put a dialable
// address in its peer ticket, so it is tested end to end rather than in halves.
#[tokio::test]
async fn a_node_is_told_the_address_it_was_seen_at() {
    let registry_dir = TempDir::new().expect("tempdir");
    let client_dir = TempDir::new().expect("tempdir");

    let registry_identity =
        NodeIdentity::load_or_create(registry_dir.path().to_str().expect("path"))
            .expect("identity");
    let client_identity =
        NodeIdentity::load_or_create(client_dir.path().to_str().expect("path")).expect("identity");

    let registry = PeerEndpoint::bind_with_alpns(
        &registry_identity,
        None,
        vec![AddressObserver::ALPN.to_vec()],
    )
    .await
    .expect("bind registry");
    let client = PeerEndpoint::bind_with_alpns(
        &client_identity,
        None,
        vec![AddressObserver::ALPN.to_vec()],
    )
    .await
    .expect("bind client");

    let listening = registry.endpoint().clone();
    let responder = tokio::spawn(async move {
        let incoming = listening.accept().await.expect("incoming");
        // Read before awaiting: this is how the connection actually arrived, and it
        // is not available once there is a `Connection`.
        let observed = incoming.remote_addr();
        let conn = incoming.await.expect("connection");
        AddressObserver::reply_to(&conn, observed)
            .await
            .expect("reply");
    });

    let addr = PeerTicket::parse(&registry.ticket().await.expect("ticket")).expect("parse");
    let observed = tokio::time::timeout(
        Duration::from_secs(10),
        AddressObserver::observe(client.endpoint(), addr),
    )
    .await
    .expect("the exchange completes within the timeout")
    .expect("observe");

    let observed = observed.expect("a same-host dial is a direct connection");

    // A concrete direct address is asserted rather than a loopback one. Iroh probes
    // every candidate a ticket carries in parallel and keeps whichever answers first,
    // so a same-host exchange may arrive over a LAN interface. What matters to a
    // server behind NAT is that it is told a real address it can advertise.
    assert!(
        !observed.ip().is_unspecified(),
        "the observed address must be concrete, got {observed}"
    );
    assert_ne!(observed.port(), 0, "the observed address must carry a port");

    responder.await.expect("join");
}

// The ALPN is a cross-version contract with every deployed server. Changing it stops
// address observation working, and the symptom is a dial that times out rather than
// one that reports a mismatch.
#[test]
fn the_observe_alpn_is_pinned() {
    assert_eq!(AddressObserver::ALPN, b"bvc-observe/1");
}

// Distinct from the peer wire's ALPN. They are dispatched apart by exactly this value.
#[test]
fn the_observe_alpn_differs_from_the_peer_alpn() {
    assert_ne!(AddressObserver::ALPN, PeerEndpoint::ALPN);
}
