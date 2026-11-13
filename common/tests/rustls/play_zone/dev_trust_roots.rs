use common::errors::PlayZoneError;
use common::rustls::{BvcClientTls, DevTrustRoots, PlayRoots};

use super::test_pki::TestPki;

const PLAY_NAME: &str = "a-b-c.play.bedrockvc.stream";

#[test]
fn a_self_signed_server_is_rejected_without_a_dev_trust_root() {
    let server = TestPki::unconstrained_root("Dev Server CA");
    let verifier = BvcClientTls::server_verifier(PlayRoots::builtin().unwrap(), vec![]).unwrap();

    let leaf = server.leaf(&["localhost"]);

    assert!(TestPki::verify_any(verifier.as_ref(), &leaf, "localhost").is_err());
}

#[test]
fn a_self_signed_server_is_accepted_once_its_root_is_a_dev_trust_root() {
    let server = TestPki::unconstrained_root("Dev Server CA");
    let path = TestPki::bundle_file("accepted", &server.root_pem());
    let dev_roots = DevTrustRoots::from_path(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    let verifier =
        BvcClientTls::server_verifier(PlayRoots::builtin().unwrap(), dev_roots).unwrap();

    let leaf = server.leaf(&["localhost"]);
    let other_name = server.leaf(&["elsewhere.test"]);

    assert!(TestPki::verify_any(verifier.as_ref(), &leaf, "localhost").is_ok());
    // The dev root adds a trust anchor and nothing else: the name is still checked.
    assert!(TestPki::verify_any(verifier.as_ref(), &other_name, "localhost").is_err());
}

#[test]
fn a_dev_trust_root_does_not_vouch_for_a_play_name() {
    let server = TestPki::unconstrained_root("Dev Server CA");
    let path = TestPki::bundle_file("play", &server.root_pem());
    let dev_roots = DevTrustRoots::from_path(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    let verifier =
        BvcClientTls::server_verifier(PlayRoots::builtin().unwrap(), dev_roots).unwrap();

    let play_leaf = server.leaf(&[PLAY_NAME]);
    let plain_leaf = server.leaf(&["localhost"]);

    assert!(TestPki::verify_any(verifier.as_ref(), &plain_leaf, "localhost").is_ok());
    assert!(TestPki::verify_any(verifier.as_ref(), &play_leaf, PLAY_NAME).is_err());
}

#[test]
fn an_unreadable_dev_trust_bundle_is_an_error() {
    let missing = std::env::temp_dir().join(format!(
        "bvc-dev-trust-roots-{}-missing.pem",
        std::process::id()
    ));

    assert!(matches!(
        DevTrustRoots::from_path(&missing),
        Err(PlayZoneError::DevRoot(_))
    ));
}

#[test]
fn a_dev_trust_bundle_without_a_certificate_is_an_error() {
    let path = TestPki::bundle_file("empty", "not a certificate\n");
    let result = DevTrustRoots::from_path(&path);
    std::fs::remove_file(&path).unwrap();

    assert!(matches!(result, Err(PlayZoneError::DevRoot(_))));
}

#[test]
fn a_dev_trust_root_the_root_store_refuses_names_the_variable() {
    // Valid PEM framing around bytes that are not a certificate.
    let path = TestPki::bundle_file(
        "refused",
        "-----BEGIN CERTIFICATE-----\nAAECAwQFBgc=\n-----END CERTIFICATE-----\n",
    );
    let result = DevTrustRoots::from_path(&path);
    std::fs::remove_file(&path).unwrap();

    let error = result.unwrap_err();
    assert!(matches!(error, PlayZoneError::DevRoot(_)));
    assert!(error.to_string().contains(DevTrustRoots::ENV));
}

#[test]
fn a_baked_pem_is_trusted_for_a_non_play_name_and_never_for_a_play_name() {
    let server = TestPki::unconstrained_root("Dev Server CA");
    let baked = DevTrustRoots::from_sources(None, Some(&server.root_pem())).unwrap();
    let verifier = BvcClientTls::server_verifier(PlayRoots::builtin().unwrap(), baked).unwrap();

    let plain_leaf = server.leaf(&["localhost"]);
    let play_leaf = server.leaf(&[PLAY_NAME]);

    assert!(TestPki::verify_any(verifier.as_ref(), &plain_leaf, "localhost").is_ok());
    assert!(TestPki::verify_any(verifier.as_ref(), &play_leaf, PLAY_NAME).is_err());
}

#[test]
fn a_bad_baked_pem_is_a_dev_root_error_naming_the_baked_variable() {
    let result = DevTrustRoots::from_sources(
        None,
        Some("-----BEGIN CERTIFICATE-----\nAAECAwQFBgc=\n-----END CERTIFICATE-----\n"),
    );

    let error = result.unwrap_err();
    assert!(matches!(error, PlayZoneError::DevRoot(_)));
    assert!(error.to_string().contains(DevTrustRoots::BAKED_ENV));
}

#[test]
fn a_baked_value_without_a_certificate_is_an_error() {
    let result = DevTrustRoots::from_sources(None, Some("not a certificate\n"));

    assert!(matches!(result, Err(PlayZoneError::DevRoot(_))));
}

#[test]
fn an_empty_baked_value_means_nothing_was_baked() {
    assert!(DevTrustRoots::from_sources(None, Some("")).unwrap().is_empty());
    assert!(DevTrustRoots::from_sources(None, None).unwrap().is_empty());
}

#[test]
fn both_sources_are_used_when_both_are_present() {
    let runtime = TestPki::unconstrained_root("Runtime CA");
    let baked = TestPki::unconstrained_root("Baked CA");
    let path = TestPki::bundle_file("both", &runtime.root_pem());
    let roots = DevTrustRoots::from_sources(Some(&path), Some(&baked.root_pem()));
    std::fs::remove_file(&path).unwrap();
    let verifier =
        BvcClientTls::server_verifier(PlayRoots::builtin().unwrap(), roots.unwrap()).unwrap();

    let from_runtime = runtime.leaf(&["localhost"]);
    let from_baked = baked.leaf(&["localhost"]);

    assert!(TestPki::verify_any(verifier.as_ref(), &from_runtime, "localhost").is_ok());
    assert!(TestPki::verify_any(verifier.as_ref(), &from_baked, "localhost").is_ok());
}

#[test]
fn an_unreadable_runtime_bundle_fails_even_when_a_baked_pem_is_good() {
    let baked = TestPki::unconstrained_root("Baked CA");
    let missing = std::env::temp_dir().join(format!(
        "bvc-dev-trust-roots-{}-missing-both.pem",
        std::process::id()
    ));

    assert!(matches!(
        DevTrustRoots::from_sources(Some(&missing), Some(&baked.root_pem())),
        Err(PlayZoneError::DevRoot(_))
    ));
}
