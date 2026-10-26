use std::time::Duration;

use bvc_relay::node::{NodeIdentity, PeerTicket};
use bvc_relay::peer::{AddressObserver, PeerEndpoint};
use tempfile::TempDir;

use crate::enroll::endpoint::registry_harness;

// Address observation is open to every server, member or not, because peering is not
// a paid feature. A node that has never enrolled must still be answered.
#[tokio::test]
async fn an_unenrolled_node_is_told_its_address() {
    let harness = registry_harness(vec!["role-a".to_string()]).await;

    let dir = TempDir::new().expect("tempdir");
    let identity =
        NodeIdentity::load_or_create(dir.path().to_str().expect("path")).expect("identity");
    let client =
        PeerEndpoint::bind_with_alpns(&identity, None, vec![AddressObserver::ALPN.to_vec()])
            .await
            .expect("bind");

    let registry = PeerTicket::parse(&harness.ticket).expect("parse");
    let observed = tokio::time::timeout(
        Duration::from_secs(10),
        AddressObserver::observe(client.endpoint(), registry),
    )
    .await
    .expect("the exchange completes within the timeout")
    .expect("observe")
    .expect("a same-host dial is a direct connection");

    assert!(!observed.ip().is_unspecified());
    assert_ne!(observed.port(), 0);
}
