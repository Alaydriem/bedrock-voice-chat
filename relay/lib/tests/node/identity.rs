use bvc_relay::node::NodeIdentity;
use tempfile::TempDir;

#[test]
fn an_identity_is_generated_on_first_load() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().to_str().expect("utf-8 path");

    let identity = NodeIdentity::load_or_create(path).expect("first load creates a key");

    assert!(
        dir.path().join("node.key").exists(),
        "the secret key must be persisted, or every restart is a new identity"
    );
    assert_eq!(identity.node_id(), identity.secret_key().public());
}

#[test]
fn the_identity_is_stable_across_loads() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().to_str().expect("utf-8 path");

    let first = NodeIdentity::load_or_create(path).expect("first load");
    let second = NodeIdentity::load_or_create(path).expect("second load");

    assert_eq!(
        first.node_id(),
        second.node_id(),
        "a restart must keep the identity every config block names"
    );
}

#[test]
fn a_corrupt_key_file_is_an_error_rather_than_a_silent_regeneration() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().to_str().expect("utf-8 path");
    std::fs::write(dir.path().join("node.key"), b"not a key").expect("write junk");

    assert!(
        NodeIdentity::load_or_create(path).is_err(),
        "silently regenerating would void every peer block naming the old id"
    );
}
