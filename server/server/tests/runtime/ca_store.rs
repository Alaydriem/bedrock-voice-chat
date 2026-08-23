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

// A first boot that stored the wrong authority — a missing volume mount, a mistyped
// certs_path, a CLI run against another directory — makes that authority the source of
// truth, and the original would then be clobbered on every later boot. The rename is what
// keeps the original recoverable.
#[tokio::test]
async fn a_disagreeing_disk_file_is_renamed_rather_than_overwritten() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().to_str().unwrap();

    let (stored_cert, _stored_key) = CaStore::ensure(&db.connection, path, &sans())
        .await
        .expect("seed the database");

    // A different authority appears on disk under the same names.
    let other = TempDir::new().expect("tempdir");
    let other_db = DatabaseFixture::create().await.expect("fixture");
    let (other_cert, other_key) =
        CaStore::ensure(&other_db.connection, other.path().to_str().unwrap(), &sans())
            .await
            .expect("other authority");
    std::fs::write(dir.path().join("ca.crt"), &other_cert).expect("write");
    std::fs::write(dir.path().join("ca.key"), &other_key).expect("write");

    let (result_cert, _result_key) = CaStore::ensure(&db.connection, path, &sans())
        .await
        .expect("ensure");

    assert_eq!(
        result_cert, stored_cert,
        "the database is the source of truth"
    );

    let superseded: Vec<String> = std::fs::read_dir(dir.path())
        .expect("read_dir")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.contains(".superseded-"))
        .collect();
    assert_eq!(
        superseded.len(),
        2,
        "both ca.crt and ca.key must be preserved, got {superseded:?}"
    );
}

// An upgrade has no row to disagree with, so it must produce no superseded file at all.
// One appearing during an upgrade is a defect, not a diagnostic.
#[tokio::test]
async fn importing_from_disk_never_supersedes_anything() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().to_str().unwrap();

    // A deployment that predates the database-backed store.
    let seed = TempDir::new().expect("tempdir");
    let seed_db = DatabaseFixture::create().await.expect("fixture");
    let (cert, key) = CaStore::ensure(&seed_db.connection, seed.path().to_str().unwrap(), &sans())
        .await
        .expect("seed");
    std::fs::write(dir.path().join("ca.crt"), &cert).expect("write");
    std::fs::write(dir.path().join("ca.key"), &key).expect("write");

    CaStore::ensure(&db.connection, path, &sans())
        .await
        .expect("upgrade");

    let superseded: Vec<String> = std::fs::read_dir(dir.path())
        .expect("read_dir")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.contains(".superseded-"))
        .collect();
    assert!(
        superseded.is_empty(),
        "an upgrade adopts the disk authority; nothing is superseded, got {superseded:?}"
    );
}

// A mismatched pair must never become the authoritative copy. Whatever produced it, the
// database is where it would become permanent.
#[tokio::test]
async fn a_mismatched_pair_is_never_stored() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().to_str().unwrap();

    // ca.crt from one authority, ca.key from another, with nothing yet in the database.
    let a = TempDir::new().expect("tempdir");
    let b = TempDir::new().expect("tempdir");
    let a_db = DatabaseFixture::create().await.expect("fixture");
    let b_db = DatabaseFixture::create().await.expect("fixture");
    let (a_cert, _a_key) = CaStore::ensure(&a_db.connection, a.path().to_str().unwrap(), &sans())
        .await
        .expect("a");
    let (_b_cert, b_key) = CaStore::ensure(&b_db.connection, b.path().to_str().unwrap(), &sans())
        .await
        .expect("b");
    std::fs::write(dir.path().join("ca.crt"), &a_cert).expect("write");
    std::fs::write(dir.path().join("ca.key"), &b_key).expect("write");

    let (cert, key) = CaStore::ensure(&db.connection, path, &sans())
        .await
        .expect("ensure repairs rather than failing");

    assert_eq!(key, b_key, "the key on disk is the trust anchor");
    assert_ne!(cert, a_cert, "the foreign certificate must be re-signed");
    assert!(
        bvc_server_lib::runtime::ca_cert::KeyMatch::matches(
            &cert,
            &KeyPair::from_pem(&key).expect("kp")
        ),
        "what reaches the database must correspond"
    );
}
