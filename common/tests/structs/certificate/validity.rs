use base64::Engine;
use common::structs::certificate::CertificateValidity;
use rcgen::{CertificateParams, KeyPair};

fn pem_wrapping(der: &[u8]) -> String {
    let body = base64::engine::general_purpose::STANDARD.encode(der);
    format!("-----BEGIN CERTIFICATE-----\n{body}\n-----END CERTIFICATE-----\n")
}

// The value feeds the revocation row's `expires_at`, which is what lets a pruner drop a row
// once the certificate could no longer be presented. Seconds, not milliseconds: a unit slip
// here would only surface years later as rows that never prune, or prune immediately.
#[test]
fn not_after_is_a_unix_timestamp_in_seconds() {
    let key_pair = KeyPair::generate().expect("keypair");
    let params = CertificateParams::default();
    let expected = params.not_after.unix_timestamp();
    let cert = params.self_signed(&key_pair).expect("self-signed cert");

    let not_after = CertificateValidity::not_after(&cert.pem()).expect("not_after");

    assert_eq!(not_after, expected);
}

// Runs over whatever is stored in the database, which may be corrupt. A PEM envelope that
// parses but holds something other than a certificate must be reported as absent rather than
// panicking or yielding a nonsense timestamp.
#[test]
fn not_after_returns_none_when_the_pem_does_not_hold_a_certificate() {
    assert_eq!(CertificateValidity::not_after(""), None);
    assert_eq!(CertificateValidity::not_after("not a certificate"), None);
    assert_eq!(
        CertificateValidity::not_after(&pem_wrapping(b"a valid envelope, invalid contents")),
        None
    );
}
