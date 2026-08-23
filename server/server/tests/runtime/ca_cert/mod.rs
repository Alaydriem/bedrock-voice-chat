mod key_match;
mod san_key_set;

use std::fs as stdfs;

use bvc_server_lib::runtime::ca_cert::{CaCertManager, KeyMatch};
use rcgen::KeyPair;
use tempfile::TempDir;

fn s(v: &str) -> String {
    v.to_string()
}

#[test]
fn ensure_creates_certs_dir_if_missing() {
    let outer = TempDir::new().unwrap();
    let nested = outer.path().join("does/not/exist/yet");
    let path = nested.to_str().unwrap();
    let mgr = CaCertManager::new(path);
    mgr.ensure(&[s("localhost")]).unwrap();
    assert!(nested.join("ca.crt").exists());
    assert!(nested.join("ca.key").exists());
}

#[test]
fn ensure_is_noop_when_sans_match() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_str().unwrap();
    let sans = vec![s("localhost"), s("127.0.0.1")];
    let mgr = CaCertManager::new(path);
    let (cert1, key1) = mgr.ensure(&sans).unwrap();
    let cert_mtime_1 = stdfs::metadata(dir.path().join("ca.crt"))
        .unwrap()
        .modified()
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(20));
    let (cert2, key2) = mgr.ensure(&sans).unwrap();
    let cert_mtime_2 = stdfs::metadata(dir.path().join("ca.crt"))
        .unwrap()
        .modified()
        .unwrap();
    assert_eq!(
        cert1, cert2,
        "cert PEM must be byte-equal across no-op runs"
    );
    assert_eq!(key1, key2, "key PEM must be byte-equal across no-op runs");
    assert_eq!(cert_mtime_1, cert_mtime_2, "ca.crt must not be rewritten");
}

#[test]
fn ensure_is_order_and_dedup_insensitive() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_str().unwrap();
    let mgr = CaCertManager::new(path);
    mgr.ensure(&[s("a.example"), s("b.example")]).unwrap();
    let cert_mtime_1 = stdfs::metadata(dir.path().join("ca.crt"))
        .unwrap()
        .modified()
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(20));
    mgr.ensure(&[s("b.example"), s("a.example"), s("a.example")])
        .unwrap();
    let cert_mtime_2 = stdfs::metadata(dir.path().join("ca.crt"))
        .unwrap()
        .modified()
        .unwrap();
    assert_eq!(
        cert_mtime_1, cert_mtime_2,
        "ca.crt must not be rewritten on equivalent SAN set"
    );
}

#[test]
fn ensure_errors_on_corrupt_existing_cert() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_str().unwrap();
    let kp = KeyPair::generate().unwrap();
    stdfs::write(dir.path().join("ca.key"), kp.serialize_pem()).unwrap();
    stdfs::write(dir.path().join("ca.crt"), "not a real PEM").unwrap();
    let mgr = CaCertManager::new(path);
    let err = mgr.ensure(&[s("localhost")]).unwrap_err();
    assert!(format!("{err}").contains("ca.crt"));
}

// A certificate and a key that do not correspond is not a harmless state.
// `Issuer::from_ca_cert_pem` does not check correspondence either, so player certificates
// get signed by one key and stamped with the other's issuer. They then fail *signature*
// verification, which reaches a client as a `decrypt_error` alert rather than `unknown_ca`.
#[test]
fn ensure_resigns_a_certificate_that_does_not_match_the_key() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_str().unwrap();
    let mgr = CaCertManager::new(path);
    let (_cert, key_pem) = mgr.ensure(&[s("localhost")]).unwrap();

    // A certificate from an unrelated authority, written over the matching one.
    let foreign_dir = TempDir::new().unwrap();
    let (foreign_cert, foreign_key) = CaCertManager::new(foreign_dir.path().to_str().unwrap())
        .ensure(&[s("localhost")])
        .unwrap();
    stdfs::write(dir.path().join("ca.crt"), &foreign_cert).unwrap();

    let (repaired_cert, repaired_key) = mgr.ensure(&[s("localhost")]).unwrap();

    assert_eq!(
        repaired_key, key_pem,
        "the keypair is the trust anchor and must never be replaced"
    );
    assert_ne!(repaired_key, foreign_key);
    assert_ne!(
        repaired_cert, foreign_cert,
        "the mismatched certificate must be re-signed, not returned"
    );
    assert!(
        KeyMatch::matches(&repaired_cert, &KeyPair::from_pem(&repaired_key).unwrap()),
        "the repaired pair must correspond"
    );
}

// A certificate inside its renewal window is re-signed with the same keypair. Without this
// a deployment older than the 90-day validity serves an expired trust anchor forever.
#[test]
fn ensure_resigns_a_certificate_inside_its_renewal_window() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_str().unwrap();
    let mgr = CaCertManager::new(path);
    let (original, key_pem) = mgr.ensure(&[s("localhost")]).unwrap();

    // Backdate the certificate by re-signing it with a validity window that has almost run
    // out, using the same keypair the manager will find on disk.
    let kp = KeyPair::from_pem(&key_pem).unwrap();
    let expiring = expiring_cert(&kp);
    stdfs::write(dir.path().join("ca.crt"), &expiring).unwrap();

    let (refreshed, refreshed_key) = mgr.ensure(&[s("localhost")]).unwrap();

    assert_eq!(refreshed_key, key_pem, "the keypair must never change");
    assert_ne!(refreshed, expiring, "an expiring certificate must be re-signed");
    assert_ne!(refreshed, original);
}

// A certificate with the same subject and SANs but only two days left to run.
fn expiring_cert(keypair: &KeyPair) -> String {
    use rcgen::{CertificateParams, DistinguishedName, ExtendedKeyUsagePurpose, IsCa, KeyUsagePurpose};
    use time::{Duration, OffsetDateTime};

    let mut params = CertificateParams::new(vec![s("localhost")]).unwrap();
    let mut dn = DistinguishedName::new();
    dn.push(rcgen::DnType::CommonName, "Bedrock Voice Chat");
    params.distinguished_name = dn;
    params.is_ca = IsCa::NoCa;
    params.use_authority_key_identifier_extension = true;
    params.key_usages = vec![KeyUsagePurpose::KeyCertSign];
    params.extended_key_usages = vec![
        ExtendedKeyUsagePurpose::ClientAuth,
        ExtendedKeyUsagePurpose::ServerAuth,
    ];
    params.not_before = OffsetDateTime::now_utc() - Duration::days(88);
    params.not_after = OffsetDateTime::now_utc() + Duration::days(2);
    params.self_signed(keypair).unwrap().pem()
}
