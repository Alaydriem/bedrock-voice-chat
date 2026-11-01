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

fn mint_cert_between(not_before: OffsetDateTime, not_after: OffsetDateTime) -> String {
    let mut params = CertificateParams::new(vec!["test.example.com".to_string()]).unwrap();
    params.not_before = not_before;
    params.not_after = not_after;
    let key = KeyPair::generate().unwrap();
    params.self_signed(&key).unwrap().pem()
}

#[test]
fn renew_at_is_two_thirds_through_a_fifteen_day_certificate() {
    let issued = OffsetDateTime::from_unix_timestamp(1_900_000_000).unwrap();
    let pem = mint_cert_between(issued, issued + time::Duration::days(15));

    let renew_at = CertificateExpiry::renew_at(&pem).unwrap();

    let expected = std::time::UNIX_EPOCH
        + Duration::from_secs((1_900_000_000 + 10 * 86400) as u64);
    assert_eq!(renew_at, expected);
}

#[test]
fn a_certificate_past_its_not_after_is_expired() {
    let pem = mint_cert(OffsetDateTime::now_utc() - time::Duration::hours(1));
    assert!(CertificateExpiry::is_expired(&pem).unwrap());
}

#[test]
fn a_certificate_inside_its_validity_is_not_expired() {
    let pem = mint_cert(OffsetDateTime::now_utc() + time::Duration::days(1));
    assert!(!CertificateExpiry::is_expired(&pem).unwrap());
}

#[test]
fn garbage_pem_is_not_a_renewal_time() {
    assert!(CertificateExpiry::renew_at("not a pem").is_err());
    assert!(CertificateExpiry::is_expired("not a pem").is_err());
}

#[test]
fn garbage_pem_is_an_error() {
    assert!(CertificateExpiry::is_valid_for("not a pem", Duration::from_secs(1)).is_err());
}
