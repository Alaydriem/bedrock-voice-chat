use bvc_server_lib::runtime::CaStore;
use rcgen::KeyPair;
use tempfile::TempDir;

use crate::harness::DatabaseFixture;

fn sans() -> Vec<String> {
    vec!["localhost".to_string(), "127.0.0.1".to_string()]
}

#[tokio::test]
async fn a_first_boot_generates_and_stores_the_authority() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().to_str().unwrap();

    assert!(!CaStore::exists(&db.connection).await.expect("exists"));

    let (cert, key) = CaStore::ensure(&db.connection, path, &sans())
        .await
        .expect("ensure");

    assert!(CaStore::exists(&db.connection).await.expect("exists"));
    assert!(!cert.is_empty() && !key.is_empty());
}

// The database is the source of truth, but the TLS stacks read file paths — Rocket's
// `tls.mutual.ca_certs`, the WebSocket trust root, and `CertificateService`. If the bytes are
// not on disk, none of them can start.
#[tokio::test]
async fn the_authority_is_materialised_to_disk() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().to_str().unwrap();

    let (cert, key) = CaStore::ensure(&db.connection, path, &sans())
        .await
        .expect("ensure");

    assert_eq!(
        std::fs::read_to_string(dir.path().join("ca.crt")).expect("ca.crt"),
        cert
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("ca.key")).expect("ca.key"),
        key
    );
}

// This is what makes the feature safe to ship. An upgrade must adopt the certificate authority
// the deployment already has; minting a fresh one would invalidate every player certificate
// ever issued by it, and every player would be locked out at once.
#[tokio::test]
async fn an_existing_on_disk_authority_is_imported_rather_than_replaced() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().to_str().unwrap();

    // Stand up a deployment that predates the database-backed store.
    let existing = KeyPair::generate().expect("keypair");
    let existing_key_pem = existing.serialize_pem();
    std::fs::write(dir.path().join("ca.key"), &existing_key_pem).expect("write ca.key");

    let (_cert, key) = CaStore::ensure(&db.connection, path, &sans())
        .await
        .expect("ensure");

    assert_eq!(
        key, existing_key_pem,
        "the existing keypair is the trust anchor and must be adopted, never replaced"
    );
}

// A restart must not mint a second authority, for the same reason.
#[tokio::test]
async fn a_second_boot_reuses_the_stored_authority() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().to_str().unwrap();

    let (first_cert, first_key) = CaStore::ensure(&db.connection, path, &sans())
        .await
        .expect("first");
    let (second_cert, second_key) = CaStore::ensure(&db.connection, path, &sans())
        .await
        .expect("second");

    assert_eq!(first_key, second_key);
    assert_eq!(first_cert, second_cert);
}

// The point of the whole change: a container gets a fresh, empty directory every start and
// still comes up with the identity it had before.
#[tokio::test]
async fn an_empty_certs_directory_is_repopulated_from_the_database() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let first_dir = TempDir::new().expect("tempdir");
    let (cert, key) = CaStore::ensure(&db.connection, first_dir.path().to_str().unwrap(), &sans())
        .await
        .expect("first boot");

    // A new container: nothing persisted, only the database.
    let second_dir = TempDir::new().expect("tempdir");
    let (restored_cert, restored_key) =
        CaStore::ensure(&db.connection, second_dir.path().to_str().unwrap(), &sans())
            .await
            .expect("second boot");

    assert_eq!(restored_key, key, "the trust anchor survives the container");
    assert_eq!(restored_cert, cert);
    assert!(second_dir.path().join("ca.key").exists());
}

// A SAN change re-signs with the same keypair, so the trust anchor is unchanged and the new
// certificate is written back rather than being regenerated on every later boot.
#[tokio::test]
async fn a_san_change_resigns_and_is_written_back() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().to_str().unwrap();

    let (first_cert, first_key) = CaStore::ensure(&db.connection, path, &sans())
        .await
        .expect("first");

    let mut widened = sans();
    widened.push("voice.example.com".to_string());
    let (second_cert, second_key) = CaStore::ensure(&db.connection, path, &widened)
        .await
        .expect("second");

    assert_eq!(first_key, second_key, "the keypair must never change");
    assert_ne!(first_cert, second_cert, "the certificate must re-sign");

    // Written back: booting again with the same SANs is a no-op.
    let (third_cert, _) = CaStore::ensure(&db.connection, path, &widened)
        .await
        .expect("third");
    assert_eq!(second_cert, third_cert);
}
