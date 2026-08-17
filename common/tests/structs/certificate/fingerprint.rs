use base64::Engine;
use common::structs::certificate::CertificateFingerprint;

// The PEM envelope is the only thing `from_pem` parses, so arbitrary bytes stand in for a
// certificate here. Using a real one would test x509-parser rather than our derivation.
fn pem_wrapping(der: &[u8]) -> String {
    let body = base64::engine::general_purpose::STANDARD.encode(der);
    format!("-----BEGIN CERTIFICATE-----\n{body}\n-----END CERTIFICATE-----\n")
}

#[test]
fn from_pem_hashes_the_same_bytes_as_from_der() {
    let der = b"\x30\x82\x01\x0a not a real certificate, only bytes";

    let from_der = CertificateFingerprint::from_der(der);
    let from_pem = CertificateFingerprint::from_pem(&pem_wrapping(der));

    assert_eq!(Some(from_der), from_pem);
}

#[test]
fn from_der_is_lowercase_hex_of_length_64() {
    let fingerprint = CertificateFingerprint::from_der(b"anything");

    assert_eq!(fingerprint.len(), 64);
    assert!(fingerprint.chars().all(|c| c.is_ascii_hexdigit()));
    assert_eq!(fingerprint, fingerprint.to_lowercase());
}

// An unparseable PEM must not degrade to hashing the raw string. The backfill leaves the
// stored column empty on a row whose PEM does not parse, and a value that matched anything
// would let an empty presented fingerprint match whichever row the database returned first.
#[test]
fn from_pem_returns_none_for_input_that_is_not_a_pem_block() {
    assert_eq!(CertificateFingerprint::from_pem(""), None);
    assert_eq!(CertificateFingerprint::from_pem("not a certificate"), None);
    assert_eq!(
        CertificateFingerprint::from_pem("-----BEGIN CERTIFICATE-----\n!!!!\n"),
        None
    );
}
