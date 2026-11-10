use bvc_relay_service::play::PlayRootPin;
use rcgen::{BasicConstraints, CertificateParams, IsCa, KeyPair};

const PRODUCTION_ROOT: &str =
    include_str!("../../../../common/resources/play_roots/bvc-play-root-2026.pem");

#[test]
fn the_production_root_is_pinned() {
    assert!(PlayRootPin::is_pinned(PRODUCTION_ROOT).expect("the root parses"));
}

#[test]
fn another_root_is_not_pinned() {
    let key = KeyPair::generate().expect("a key pair");
    let mut params = CertificateParams::new(vec!["ca.example.test".to_string()]).expect("params");
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    let other = params.self_signed(&key).expect("a self-signed CA").pem();

    assert!(!PlayRootPin::is_pinned(&other).expect("the root parses"));
    // One pinned root does not vouch for a bundle that also holds an unpinned one.
    assert!(!PlayRootPin::is_pinned(&format!("{PRODUCTION_ROOT}\n{other}")).expect("parses"));
}

#[test]
fn an_empty_root_is_not_pinned() {
    assert!(!PlayRootPin::is_pinned("").expect("an empty file parses"));
}
