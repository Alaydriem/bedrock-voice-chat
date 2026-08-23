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

// The server holds this key in the database and never writes it to disk, so the identity
// has to be constructible from bytes alone.
#[test]
fn an_identity_built_from_bytes_matches_one_loaded_from_a_file() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().to_str().expect("utf-8 path");

    let from_file = NodeIdentity::load_or_create(path).expect("load_or_create");
    let bytes = from_file.secret_bytes();
    let from_bytes = NodeIdentity::from_secret_bytes(&bytes);

    assert_eq!(
        from_bytes.node_id(),
        from_file.node_id(),
        "the node id is what a far-side peer block names; it must survive the round trip"
    );
}

#[test]
fn secret_bytes_round_trips() {
    let original = [7u8; 32];
    let identity = NodeIdentity::from_secret_bytes(&original);

    assert_eq!(identity.secret_bytes(), original);
}
