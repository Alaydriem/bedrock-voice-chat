use std::sync::Arc;
use std::time::Duration;

use bvc_server_lib::services::acme::AcmeStorage;
use rcgen::{CertificateParams, KeyPair};
use tempfile::TempDir;
use time::OffsetDateTime;

use crate::harness::DatabaseFixture;

const DIRECTORY: &str = "https://acme-v02.api.letsencrypt.org/directory";

fn names() -> Vec<String> {
    vec!["test.example.com".to_string()]
}

fn mint_cert(days_remaining: i64) -> String {
    let mut params = CertificateParams::new(vec!["test.example.com".to_string()]).unwrap();
    params.not_before = OffsetDateTime::now_utc() - time::Duration::days(1);
    params.not_after = OffsetDateTime::now_utc() + time::Duration::days(days_remaining);
    let key = KeyPair::generate().unwrap();
    params.self_signed(&key).unwrap().pem()
}

fn storage(dir: &TempDir, db: &DatabaseFixture, names: Vec<String>) -> AcmeStorage {
    AcmeStorage::new(
        dir.path().to_str().unwrap(),
        Arc::new(db.connection.clone()),
        DIRECTORY.to_string(),
        names,
    )
}

#[tokio::test]
async fn load_returns_none_when_no_certificate_stored() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let dir = TempDir::new().unwrap();
    let storage = storage(&dir, &db, names());

    assert!(
        storage
            .load_certificate_valid_for(Duration::from_secs(86400))
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn store_then_load_round_trips_when_valid() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let dir = TempDir::new().unwrap();
    let storage = storage(&dir, &db, names());
    storage.store_account_credentials("{}").await.unwrap();
    let cert = mint_cert(60);

    storage.store_certificate(&cert, "key-pem").await.unwrap();

    let loaded = storage
        .load_certificate_valid_for(Duration::from_secs(30 * 86400))
        .await
        .unwrap();
    assert_eq!(loaded.as_deref(), Some(cert.as_str()));
    assert!(storage.certificate_path().exists());
    assert!(storage.key_path().exists());
}

#[tokio::test]
async fn load_returns_none_inside_renewal_window() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let dir = TempDir::new().unwrap();
    let storage = storage(&dir, &db, names());
    storage.store_account_credentials("{}").await.unwrap();

    storage
        .store_certificate(&mint_cert(10), "key-pem")
        .await
        .unwrap();

    assert!(
        storage
            .load_certificate_valid_for(Duration::from_secs(30 * 86400))
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn account_credentials_persist() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let dir = TempDir::new().unwrap();
    let storage = storage(&dir, &db, names());

    assert!(storage.load_account_credentials().await.unwrap().is_none());
    storage
        .store_account_credentials("{\"id\":1}")
        .await
        .unwrap();

    assert_eq!(
        storage.load_account_credentials().await.unwrap().as_deref(),
        Some("{\"id\":1}")
    );
}

// The whole point: a new container with an empty directory still has the account, so it does
// not re-register and consume an ACME registration.
#[tokio::test]
async fn a_new_directory_still_sees_the_stored_account() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let first = TempDir::new().unwrap();
    storage(&first, &db, names())
        .store_account_credentials("{\"id\":1}")
        .await
        .unwrap();

    let second = TempDir::new().unwrap();
    let restored = storage(&second, &db, names())
        .load_account_credentials()
        .await
        .unwrap();

    assert_eq!(restored.as_deref(), Some("{\"id\":1}"));
}

// Rocket's tls.certs and tls.key are paths, so a boot that serves the stored certificate has
// to leave it on disk even when the directory started empty.
#[tokio::test]
async fn loading_from_an_empty_directory_materialises_both_files() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let first = TempDir::new().unwrap();
    let seed = storage(&first, &db, names());
    seed.store_account_credentials("{}").await.unwrap();
    let cert = mint_cert(60);
    seed.store_certificate(&cert, "key-pem").await.unwrap();

    let second = TempDir::new().unwrap();
    let restored = storage(&second, &db, names());
    let loaded = restored
        .load_certificate_valid_for(Duration::from_secs(30 * 86400))
        .await
        .unwrap();

    assert_eq!(loaded.as_deref(), Some(cert.as_str()));
    assert_eq!(
        std::fs::read_to_string(restored.certificate_path()).unwrap(),
        cert
    );
    assert_eq!(
        std::fs::read_to_string(restored.key_path()).unwrap(),
        "key-pem"
    );
}

// A domain change must invalidate the stored certificate, or the server serves one issued for
// the wrong names. Reported as absent so the existing issuance path re-issues.
#[tokio::test]
async fn a_names_change_invalidates_the_stored_certificate() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let dir = TempDir::new().unwrap();
    let original = storage(&dir, &db, names());
    original.store_account_credentials("{}").await.unwrap();
    original
        .store_certificate(&mint_cert(60), "key-pem")
        .await
        .unwrap();

    let widened = storage(
        &dir,
        &db,
        vec![
            "test.example.com".to_string(),
            "other.example.com".to_string(),
        ],
    );

    assert!(
        widened
            .load_certificate_valid_for(Duration::from_secs(30 * 86400))
            .await
            .unwrap()
            .is_none(),
        "a certificate issued for different names must not be served"
    );
    assert_eq!(
        widened.load_account_credentials().await.unwrap().as_deref(),
        Some("{}"),
        "a names change re-issues against the same account rather than registering again"
    );
}

// The order the names arrive in is not a change.
#[tokio::test]
async fn reordering_the_names_is_not_a_change() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let dir = TempDir::new().unwrap();
    let ordered = vec!["a.example.com".to_string(), "b.example.com".to_string()];
    let reversed = vec!["b.example.com".to_string(), "a.example.com".to_string()];

    let first = storage(&dir, &db, ordered);
    first.store_account_credentials("{}").await.unwrap();
    let cert = mint_cert(60);
    first.store_certificate(&cert, "key-pem").await.unwrap();

    let second = storage(&dir, &db, reversed);

    assert_eq!(
        second
            .load_certificate_valid_for(Duration::from_secs(30 * 86400))
            .await
            .unwrap()
            .as_deref(),
        Some(cert.as_str())
    );
}

// The upgrade path: beta.20 kept all three files under <certs_path>/acme/.
#[tokio::test]
async fn existing_acme_files_are_imported() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let dir = TempDir::new().unwrap();
    let acme_dir = dir.path().join("acme");
    std::fs::create_dir_all(&acme_dir).unwrap();
    let cert = mint_cert(60);
    std::fs::write(acme_dir.join("account.json"), "{\"legacy\":true}").unwrap();
    std::fs::write(acme_dir.join("cert.pem"), &cert).unwrap();
    std::fs::write(acme_dir.join("key.pem"), "legacy-key").unwrap();

    let storage = storage(&dir, &db, names());
    storage.import_legacy().await.unwrap();

    assert_eq!(
        storage.load_account_credentials().await.unwrap().as_deref(),
        Some("{\"legacy\":true}"),
        "re-registering would consume an ACME registration for no reason"
    );
    assert_eq!(
        storage
            .load_certificate_valid_for(Duration::from_secs(30 * 86400))
            .await
            .unwrap()
            .as_deref(),
        Some(cert.as_str()),
        "re-issuing would spend one of five duplicate certificates per week"
    );
}

// Importing must never displace a stored account.
#[tokio::test]
async fn import_does_nothing_when_a_row_already_exists() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let dir = TempDir::new().unwrap();
    let storage = storage(&dir, &db, names());
    storage
        .store_account_credentials("{\"stored\":true}")
        .await
        .unwrap();

    let acme_dir = dir.path().join("acme");
    std::fs::create_dir_all(&acme_dir).unwrap();
    std::fs::write(acme_dir.join("account.json"), "{\"legacy\":true}").unwrap();

    storage.import_legacy().await.unwrap();

    assert_eq!(
        storage.load_account_credentials().await.unwrap().as_deref(),
        Some("{\"stored\":true}")
    );
}

// A certificate without its key is unusable. Storing half of it would report a certificate
// that cannot serve.
#[tokio::test]
async fn import_skips_a_certificate_missing_its_key() {
    let db = DatabaseFixture::create().await.expect("fixture");
    let dir = TempDir::new().unwrap();
    let acme_dir = dir.path().join("acme");
    std::fs::create_dir_all(&acme_dir).unwrap();
    std::fs::write(acme_dir.join("account.json"), "{}").unwrap();
    std::fs::write(acme_dir.join("cert.pem"), mint_cert(60)).unwrap();

    let storage = storage(&dir, &db, names());
    storage.import_legacy().await.unwrap();

    assert_eq!(
        storage.load_account_credentials().await.unwrap().as_deref(),
        Some("{}"),
        "the account is still worth keeping"
    );
    assert!(
        storage
            .load_certificate_valid_for(Duration::from_secs(30 * 86400))
            .await
            .unwrap()
            .is_none()
    );
}
