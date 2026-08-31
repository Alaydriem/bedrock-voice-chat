use bvc_relay::node::{NodeIdentity, PeerTicket};
use bvc_relay::peer::PeerEndpoint;
use tempfile::TempDir;

// Nothing in this crate can configure a relay. A ticket that carried a relay URL
// would mean an endpoint had one, which would mean traffic could be proxied through
// it — the single property this whole change exists to guarantee.
#[tokio::test]
async fn a_minted_ticket_never_carries_a_relay_url() {
    let dir = TempDir::new().expect("tempdir");
    let identity =
        NodeIdentity::load_or_create(dir.path().to_str().expect("path")).expect("identity");

    let endpoint = PeerEndpoint::bind(&identity).await.expect("bind");
    let addr = PeerTicket::parse(&endpoint.ticket().await.expect("ticket")).expect("parse");

    assert_eq!(
        addr.relay_urls().next(),
        None,
        "an endpoint that can name a relay can proxy through it"
    );
}

// Binding still works with no relay to fall back on, which is the ordinary case and
// now the only one.
#[tokio::test]
async fn an_endpoint_binds_without_a_relay() {
    let dir = TempDir::new().expect("tempdir");
    let identity =
        NodeIdentity::load_or_create(dir.path().to_str().expect("path")).expect("identity");

    let endpoint = PeerEndpoint::bind(&identity).await.expect("bind");

    assert_eq!(endpoint.node_id(), identity.node_id());
}
