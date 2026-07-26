use std::time::Duration;

use bvc_server_lib::services::acme::AcmeStorage;
use rcgen::{CertificateParams, KeyPair};
use tempfile::TempDir;
use time::OffsetDateTime;

fn mint_cert(days_remaining: i64) -> String {
    let mut params = CertificateParams::new(vec!["test.example.com".to_string()]).unwrap();
    params.not_before = OffsetDateTime::now_utc() - time::Duration::days(1);
    params.not_after = OffsetDateTime::now_utc() + time::Duration::days(days_remaining);
    let key = KeyPair::generate().unwrap();
    params.self_signed(&key).unwrap().pem()
}

#[test]
fn load_returns_none_when_no_certificate_stored() {
    let dir = TempDir::new().unwrap();
    let storage = AcmeStorage::new(dir.path().to_str().unwrap());
    assert!(
        storage
            .load_certificate_valid_for(Duration::from_secs(86400))
            .unwrap()
            .is_none()
    );
}

#[test]
fn store_then_load_round_trips_when_valid() {
    let dir = TempDir::new().unwrap();
    let storage = AcmeStorage::new(dir.path().to_str().unwrap());
    let cert = mint_cert(60);
    storage.store_certificate(&cert, "key-pem").unwrap();

    let loaded = storage
        .load_certificate_valid_for(Duration::from_secs(30 * 86400))
        .unwrap();
    assert_eq!(loaded.as_deref(), Some(cert.as_str()));
    assert!(storage.certificate_path().exists());
    assert!(storage.key_path().exists());
}

#[test]
fn load_returns_none_inside_renewal_window() {
    let dir = TempDir::new().unwrap();
    let storage = AcmeStorage::new(dir.path().to_str().unwrap());
    storage.store_certificate(&mint_cert(10), "key-pem").unwrap();
    assert!(
        storage
            .load_certificate_valid_for(Duration::from_secs(30 * 86400))
            .unwrap()
            .is_none()
    );
}

#[test]
fn account_credentials_persist() {
    let dir = TempDir::new().unwrap();
    let storage = AcmeStorage::new(dir.path().to_str().unwrap());
    assert!(storage.load_account_credentials().unwrap().is_none());
    storage.store_account_credentials("{\"id\":1}").unwrap();
    assert_eq!(
        storage.load_account_credentials().unwrap().as_deref(),
        Some("{\"id\":1}")
    );
}
