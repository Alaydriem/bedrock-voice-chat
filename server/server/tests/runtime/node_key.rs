use bvc_relay::node::NodeIdentity;
use bvc_server_lib::runtime::{NodeKeyStore, SecretName, SecretStore};
use tempfile::TempDir;

use crate::harness::DatabaseFixture;

#[tokio::test]
async fn a_generated_key_is_stored_as_hex_and_never_written_to_disk() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let dir = TempDir::new().expect("tempdir");
    let store = NodeKeyStore::new(dir.path().to_str().unwrap());

    let bytes = store.resolve(&db.connection).await.expect("resolve");

    let stored = SecretStore::read(&db.connection, SecretName::RelayNodeKey)
        .await
        .expect("read")
        .expect("a row");
    assert_eq!(stored.len(), 64, "32 bytes as lowercase hex");
    assert_eq!(hex::decode(&stored).expect("hex"), bytes.to_vec());
    assert!(
        !dir.path().join("node.key").exists(),
        "the database is the only durable copy"
    );
}

// The upgrade path. Every far-side `peer` block names this node's public key, so a fresh
// key would revoke this server everywhere at once.
#[tokio::test]
async fn an_existing_node_key_file_is_imported_and_preserves_the_node_id() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let dir = TempDir::new().expect("tempdir");
    let legacy = NodeIdentity::load_or_create(dir.path().to_str().unwrap()).expect("legacy");
    let expected_node_id = legacy.node_id();

    let store = NodeKeyStore::new(dir.path().to_str().unwrap());
    let bytes = store.resolve(&db.connection).await.expect("resolve");

    assert_eq!(
        NodeIdentity::from_secret_bytes(&bytes).node_id(),
        expected_node_id,
        "the peer identity must survive the upgrade"
    );
}

#[tokio::test]
async fn a_second_boot_reuses_the_stored_key() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let dir = TempDir::new().expect("tempdir");
    let store = NodeKeyStore::new(dir.path().to_str().unwrap());

    let first = store.resolve(&db.connection).await.expect("first");
    let second = store.resolve(&db.connection).await.expect("second");

    assert_eq!(first, second);
}

// A container with an empty directory still comes up with the identity it published.
#[tokio::test]
async fn an_empty_directory_is_served_from_the_database() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let first_dir = TempDir::new().expect("tempdir");
    let first = NodeKeyStore::new(first_dir.path().to_str().unwrap())
        .resolve(&db.connection)
        .await
        .expect("first boot");

    let second_dir = TempDir::new().expect("tempdir");
    let second = NodeKeyStore::new(second_dir.path().to_str().unwrap())
        .resolve(&db.connection)
        .await
        .expect("second boot");

    assert_eq!(first, second);
}

// A malformed row is an error, never a cue to mint a fresh identity — the same rule the
// file-based loader already applies. Regenerating would silently revoke this node.
#[tokio::test]
async fn a_malformed_row_is_an_error_rather_than_a_fresh_key() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let dir = TempDir::new().expect("tempdir");
    SecretStore::resolve(
        &db.connection,
        SecretName::RelayNodeKey,
        Some("not-hex"),
        None,
        || unreachable!(),
    )
    .await
    .expect("seed");

    let result = NodeKeyStore::new(dir.path().to_str().unwrap())
        .resolve(&db.connection)
        .await;

    assert!(result.is_err());
}

// The stored key outranks a file left behind by an earlier release. The file is never
// written again, so a disagreement means the database moved on and the file is stale.
#[tokio::test]
async fn a_stored_key_outranks_a_leftover_file() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let dir = TempDir::new().expect("tempdir");

    let stored = NodeKeyStore::new(dir.path().to_str().unwrap())
        .resolve(&db.connection)
        .await
        .expect("seed the database");

    // An unrelated node.key appears in the directory afterwards.
    NodeIdentity::load_or_create(dir.path().to_str().unwrap()).expect("legacy file");

    let resolved = NodeKeyStore::new(dir.path().to_str().unwrap())
        .resolve(&db.connection)
        .await
        .expect("resolve");

    assert_eq!(resolved, stored);
}
