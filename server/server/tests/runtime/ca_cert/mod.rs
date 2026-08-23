mod san_key_set;

use std::fs as stdfs;

use bvc_server_lib::runtime::ca_cert::CaCertManager;
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
