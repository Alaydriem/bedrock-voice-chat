use std::net::SocketAddr;

use bvc_relay::node::{NodeIdentity, PeerTicket};
use bvc_relay::peer::PeerEndpoint;
use tempfile::TempDir;

async fn endpoint(dir: &TempDir) -> PeerEndpoint {
    let identity =
        NodeIdentity::load_or_create(dir.path().to_str().expect("path")).expect("identity");
    PeerEndpoint::bind(&identity).await.expect("bind")
}

// A server behind NAT sees only its LAN address, so the address it was observed at
// has to reach the ticket or nobody outside can dial it. With no relay to fall back
// on, an address missing from the ticket is a path that does not exist.
#[tokio::test]
async fn an_advertised_address_reaches_the_ticket() {
    let dir = TempDir::new().expect("tempdir");
    let endpoint = endpoint(&dir).await;
    let advertised: SocketAddr = "203.0.113.10:28284".parse().expect("addr");

    let ticket = endpoint
        .ticket_advertising(Some(advertised))
        .await
        .expect("ticket");

    let addr = PeerTicket::parse(&ticket).expect("parse");
    assert!(
        addr.ip_addrs().any(|socket| *socket == advertised),
        "the advertised address must be dialable from the ticket: {addr:?}"
    );
}

// Advertising adds to what iroh already reports rather than replacing it. The
// loopback entry is what keeps a same-host bridge on `lo`, and losing it would push
// that traffic onto a LAN interface that may not exist.
#[tokio::test]
async fn advertising_keeps_the_locally_observed_addresses() {
    let dir = TempDir::new().expect("tempdir");
    let endpoint = endpoint(&dir).await;
    let advertised: SocketAddr = "203.0.113.10:28284".parse().expect("addr");

    let plain = PeerTicket::parse(&endpoint.ticket().await.expect("ticket")).expect("parse");
    let with_advertised = PeerTicket::parse(
        &endpoint
            .ticket_advertising(Some(advertised))
            .await
            .expect("ticket"),
    )
    .expect("parse");

    for local in plain.ip_addrs().copied() {
        assert!(
            with_advertised.ip_addrs().any(|socket| *socket == local),
            "advertising dropped {local}"
        );
    }
}

// Passing nothing is exactly the old behaviour, so a server that cannot reach the
// registry still mints the ticket it always did.
#[tokio::test]
async fn advertising_nothing_matches_the_plain_ticket() {
    let dir = TempDir::new().expect("tempdir");
    let endpoint = endpoint(&dir).await;

    let plain = PeerTicket::parse(&endpoint.ticket().await.expect("ticket")).expect("parse");
    let none = PeerTicket::parse(&endpoint.ticket_advertising(None).await.expect("ticket"))
        .expect("parse");

    let mut plain: Vec<SocketAddr> = plain.ip_addrs().copied().collect();
    let mut none: Vec<SocketAddr> = none.ip_addrs().copied().collect();
    plain.sort();
    none.sort();
    assert_eq!(plain, none);
}
