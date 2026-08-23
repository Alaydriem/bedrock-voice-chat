use bvc_server_lib::runtime::ca_cert::{CaCertManager, KeyMatch};
use rcgen::KeyPair;
use tempfile::TempDir;

fn s(v: &str) -> String {
    v.to_string()
}

#[test]
fn a_matching_pair_is_reported_as_matching() {
    let dir = TempDir::new().unwrap();
    let (cert, key) = CaCertManager::new(dir.path().to_str().unwrap())
        .ensure(&[s("localhost")])
        .unwrap();

    assert!(KeyMatch::matches(&cert, &KeyPair::from_pem(&key).unwrap()));
}

#[test]
fn a_certificate_from_another_key_does_not_match() {
    let first = TempDir::new().unwrap();
    let second = TempDir::new().unwrap();
    let (cert, _key) = CaCertManager::new(first.path().to_str().unwrap())
        .ensure(&[s("localhost")])
        .unwrap();
    let (_other_cert, other_key) = CaCertManager::new(second.path().to_str().unwrap())
        .ensure(&[s("localhost")])
        .unwrap();

    assert!(!KeyMatch::matches(
        &cert,
        &KeyPair::from_pem(&other_key).unwrap()
    ));
}

// A certificate that cannot be read is not one that corresponds, and the caller's answer to
// both is the same: re-sign from the key.
#[test]
fn an_unparseable_certificate_does_not_match() {
    let dir = TempDir::new().unwrap();
    let (_cert, key) = CaCertManager::new(dir.path().to_str().unwrap())
        .ensure(&[s("localhost")])
        .unwrap();

    assert!(!KeyMatch::matches(
        "not a certificate",
        &KeyPair::from_pem(&key).unwrap()
    ));
}
