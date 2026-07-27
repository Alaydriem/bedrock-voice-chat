use std::time::Duration;

use bvc_server_lib::services::acme::CertificateExpiry;
use rcgen::{CertificateParams, KeyPair};
use time::OffsetDateTime;

fn mint_cert(not_after: OffsetDateTime) -> String {
    let mut params = CertificateParams::new(vec!["test.example.com".to_string()]).unwrap();
    params.not_before = OffsetDateTime::now_utc() - time::Duration::days(1);
    params.not_after = not_after;
    let key = KeyPair::generate().unwrap();
    params.self_signed(&key).unwrap().pem()
}

#[test]
fn long_lived_cert_is_valid() {
    let pem = mint_cert(OffsetDateTime::now_utc() + time::Duration::days(60));
    assert!(CertificateExpiry::is_valid_for(&pem, Duration::from_secs(30 * 86400)).unwrap());
}

#[test]
fn cert_inside_renewal_window_is_not_valid() {
    let pem = mint_cert(OffsetDateTime::now_utc() + time::Duration::days(10));
    assert!(!CertificateExpiry::is_valid_for(&pem, Duration::from_secs(30 * 86400)).unwrap());
}

#[test]
fn garbage_pem_is_an_error() {
    assert!(CertificateExpiry::is_valid_for("not a pem", Duration::from_secs(1)).is_err());
}
