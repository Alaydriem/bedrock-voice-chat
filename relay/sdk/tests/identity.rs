use bvc_relay::node::PeerTicket;
use bvc_relay_sdk::BvcIdentity;
use tempfile::TempDir;

// An operator pastes this link into `config.hcl` once. A key that changed per start
// would revoke the bridge on every restart, and the symptom — a peer that handshakes
// and is refused — looks like a typo in a value the operator did not touch.
#[test]
fn a_peerlink_is_stable_across_reopens() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().to_str().expect("path").to_string();

    let first = BvcIdentity::open(path.clone()).expect("open").peerlink();
    let second = BvcIdentity::open(path).expect("open").peerlink();

    assert_eq!(first.expect("link"), second.expect("link"));
}

// The granting server parses this to learn which key may peer, so it has to parse
// back to the same node the identity names.
#[test]
fn a_peerlink_parses_back_to_this_node() {
    let dir = TempDir::new().expect("tempdir");
    let identity = BvcIdentity::open(dir.path().to_str().expect("path").to_string()).expect("open");

    let parsed = PeerTicket::parse(&identity.peerlink().expect("link")).expect("parse");

    assert_eq!(parsed.id.to_string(), identity.node_id());
}

// Two bridges must not be the same peer: a shared identity would let either one's
// grant authorize the other.
#[test]
fn two_directories_are_two_identities() {
    let a = TempDir::new().expect("tempdir");
    let b = TempDir::new().expect("tempdir");

    let one = BvcIdentity::open(a.path().to_str().expect("path").to_string()).expect("open");
    let two = BvcIdentity::open(b.path().to_str().expect("path").to_string()).expect("open");

    assert_ne!(one.node_id(), two.node_id());
}
