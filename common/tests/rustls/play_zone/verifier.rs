use common::rustls::PlayRoots;
use rustls::pki_types::{ServerName, UnixTime};
use sha2::Digest;
use x509_parser::extensions::GeneralName;

use super::test_pki::TestPki;

const PLAY_NAME: &str = "a-b-c.play.bedrockvc.stream";

#[test]
fn a_play_name_accepts_a_chain_to_a_play_root() {
    let play = TestPki::play_root("Test Play Root");
    let public = TestPki::unconstrained_root("Test Public Root");
    let verifier = TestPki::verifier(
        vec![play.root()],
        TestPki::fallback_trusting(vec![public.root()]),
    );

    let leaf = play.leaf(&[PLAY_NAME]);

    assert!(TestPki::verify(&verifier, &leaf, PLAY_NAME).is_ok());
}

#[test]
fn a_play_name_rejects_a_webpki_valid_chain() {
    let play = TestPki::play_root("Test Play Root");
    let public = TestPki::unconstrained_root("Test Public Root");
    let verifier = TestPki::verifier(
        vec![play.root()],
        TestPki::fallback_trusting(vec![public.root()]),
    );

    let leaf = public.leaf(&[PLAY_NAME]);

    // The fallback alone would accept this chain; the play name must not reach it.
    let control = TestPki::fallback_trusting(vec![public.root()]);
    assert!(
        control
            .verify_server_cert(
                &leaf,
                &[],
                &ServerName::try_from(PLAY_NAME).unwrap(),
                &[],
                UnixTime::now()
            )
            .is_ok()
    );
    assert!(TestPki::verify(&verifier, &leaf, PLAY_NAME).is_err());
}

#[test]
fn a_non_play_name_rejects_a_chain_to_a_play_root() {
    let play = TestPki::unconstrained_root("Test Play Root");
    let public = TestPki::unconstrained_root("Test Public Root");
    // The play root is in the fallback too, as if someone had added it by mistake.
    let verifier = TestPki::verifier(
        vec![play.root()],
        TestPki::fallback_trusting(vec![public.root(), play.root()]),
    );

    let public_leaf = public.leaf(&["example.test"]);
    let play_leaf = play.leaf(&["example.test"]);

    assert!(TestPki::verify(&verifier, &public_leaf, "example.test").is_ok());
    assert!(TestPki::verify(&verifier, &play_leaf, "example.test").is_err());
}

#[test]
fn a_public_chain_with_an_unparsable_certificate_is_accepted() {
    let play = TestPki::play_root("Test Play Root");
    let public = TestPki::unconstrained_root("Test Public Root");
    let fallback = TestPki::fallback_trusting(vec![public.root()]);
    let verifier = TestPki::verifier(vec![play.root()], fallback.clone());

    let leaf = public.leaf(&["example.test"]);
    let unparsable = rustls::pki_types::CertificateDer::from(vec![0x30, 0x03, 0x01, 0x02, 0x03]);
    let name = ServerName::try_from("example.test").unwrap();

    // The fallback alone accepts the chain; the extra certificate must not turn that into a
    // refusal.
    assert!(
        fallback
            .verify_server_cert(
                &leaf,
                std::slice::from_ref(&unparsable),
                &name,
                &[],
                UnixTime::now()
            )
            .is_ok()
    );
    assert!(
        TestPki::verify_via(
            &verifier,
            &leaf,
            std::slice::from_ref(&unparsable),
            "example.test"
        )
        .is_ok()
    );
}

#[test]
fn a_leaf_with_a_san_outside_the_zone_is_rejected_even_if_signed_by_a_play_root() {
    // Unconstrained, so only the verifier's own SAN check can refuse the second leaf.
    let play = TestPki::unconstrained_root("Test Play Root");
    let public = TestPki::unconstrained_root("Test Public Root");
    let verifier = TestPki::verifier(
        vec![play.root()],
        TestPki::fallback_trusting(vec![public.root()]),
    );

    let inside = play.leaf(&[PLAY_NAME]);
    let straddling = play.leaf(&[PLAY_NAME, "example.test"]);
    let wildcard = play.leaf(&["*.play.bedrockvc.stream"]);

    assert!(TestPki::verify(&verifier, &inside, PLAY_NAME).is_ok());
    assert!(TestPki::verify(&verifier, &straddling, PLAY_NAME).is_err());
    assert!(TestPki::verify(&verifier, &wildcard, "x.play.bedrockvc.stream").is_err());
}

#[test]
fn a_leaf_with_an_ip_san_is_rejected() {
    // Unconstrained, so only the verifier's own SAN check can refuse it.
    let play = TestPki::unconstrained_root("Test Play Root");
    let public = TestPki::unconstrained_root("Test Public Root");
    let verifier = TestPki::verifier(
        vec![play.root()],
        TestPki::fallback_trusting(vec![public.root()]),
    );

    let leaf = play.leaf(&[PLAY_NAME, "127.0.0.1"]);

    assert!(TestPki::verify(&verifier, &leaf, PLAY_NAME).is_err());
}

#[test]
fn mixed_case_play_names_are_play_names() {
    // The trailing dot is how a fully qualified name arrives. Without the trim it would
    // fall through to the public roots.
    for name in [
        "Brave-Otter-Mesa.PLAY.bedrockvc.stream",
        "Brave-Otter-Mesa.PLAY.bedrockvc.stream.",
    ] {
        let name = ServerName::try_from(name).unwrap();
        assert!(PlayRoots::is_play_name(&name), "{name:?}");
    }
}

#[test]
fn a_play_name_accepts_a_chain_through_a_play_intermediate() {
    let root = TestPki::play_root("Test Play Root");
    let intermediate = root.intermediate("Test Play Intermediate", true);
    let public = TestPki::unconstrained_root("Test Public Root");
    let verifier = TestPki::verifier(
        vec![root.root()],
        TestPki::fallback_trusting(vec![public.root()]),
    );

    let leaf = intermediate.leaf(&[PLAY_NAME]);

    assert!(TestPki::verify_via(&verifier, &leaf, &[intermediate.root()], PLAY_NAME).is_ok());
}

#[test]
fn a_non_play_name_rejects_a_leaf_signed_by_a_play_intermediate() {
    // The play root is in the fallback too, so only the verifier's own check of the
    // presented intermediate can refuse. Nothing is name-constrained.
    let root = TestPki::unconstrained_root("Test Play Root");
    let intermediate = root.intermediate("Test Play Intermediate", false);
    let verifier = TestPki::verifier(
        vec![root.root()],
        TestPki::fallback_trusting(vec![root.root()]),
    );

    let leaf = intermediate.leaf(&["example.test"]);

    // The fallback alone accepts this chain.
    assert!(
        TestPki::fallback_trusting(vec![root.root()])
            .verify_server_cert(
                &leaf,
                &[intermediate.root()],
                &ServerName::try_from("example.test").unwrap(),
                &[],
                UnixTime::now()
            )
            .is_ok()
    );
    assert!(
        TestPki::verify_via(&verifier, &leaf, &[intermediate.root()], "example.test").is_err()
    );
}

#[test]
fn with_no_play_roots_play_names_are_refused_and_other_names_use_the_fallback() {
    let public = TestPki::unconstrained_root("Test Public Root");
    let verifier = TestPki::verifier(vec![], TestPki::fallback_trusting(vec![public.root()]));

    let play_leaf = public.leaf(&[PLAY_NAME]);
    let other_leaf = public.leaf(&["example.test"]);

    assert!(TestPki::verify(&verifier, &play_leaf, PLAY_NAME).is_err());
    assert!(TestPki::verify(&verifier, &other_leaf, "example.test").is_ok());
}

#[test]
fn the_zone_apex_is_not_a_play_name() {
    for name in [
        "play.bedrockvc.stream",
        "PLAY.bedrockvc.stream",
        "notplay.bedrockvc.stream",
        "a-b-c.play.bedrockvc.stream.example.test",
        "127.0.0.1",
    ] {
        let name = ServerName::try_from(name).unwrap();
        assert!(!PlayRoots::is_play_name(&name), "{name:?}");
    }
}

// A built-in root without its name constraint would let the BVC play CA vouch for any
// name at all, so this is checked on the shipped bytes. Every permitted subtree must be the
// play zone, and every IP address must be excluded.
#[test]
fn builtin_roots_parse_and_are_name_constrained() {
    let roots = PlayRoots::builtin().unwrap();
    let expected = if cfg!(debug_assertions) { 2 } else { 1 };
    assert_eq!(roots.len(), expected);

    for root in roots {
        let (_, cert) = x509_parser::parse_x509_certificate(root.as_ref()).unwrap();
        let constraints = cert.tbs_certificate.name_constraints().unwrap().unwrap();

        assert!(constraints.critical);
        let permitted = constraints.value.permitted_subtrees.as_ref().unwrap();
        assert!(!permitted.is_empty());
        assert!(permitted.iter().all(|subtree| matches!(
            &subtree.base,
            GeneralName::DNSName(name) if *name == "play.bedrockvc.stream"
        )));

        let excluded = constraints.value.excluded_subtrees.as_ref().unwrap();
        for all_addresses in [&[0u8; 8][..], &[0u8; 32][..]] {
            assert!(excluded.iter().any(|subtree| matches!(
                &subtree.base,
                GeneralName::IPAddress(range) if *range == all_addresses
            )));
        }
    }
}

// The production root is a trust anchor for every installed client. Replacing it by
// accident would strand every play-named server, so its exact bytes are pinned here.
#[test]
fn the_production_root_is_pinned_in_every_build() {
    const PRODUCTION_SHA256: &str =
        "55e2bfb347175f4832875a674d1f5725bbad44950ec97f0340461153057e7960";

    let found = PlayRoots::builtin().unwrap().iter().any(|root| {
        let digest = sha2::Sha256::digest(root.as_ref());
        digest.iter().map(|b| format!("{b:02x}")).collect::<String>() == PRODUCTION_SHA256
    });

    assert!(found, "the production play root is missing or changed");
}
