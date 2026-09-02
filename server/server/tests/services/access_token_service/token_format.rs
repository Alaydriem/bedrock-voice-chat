use bvc_server_lib::services::TokenFormat;

#[test]
fn a_minted_token_parses_back_to_its_parts() {
    let (id, secret) = TokenFormat::mint();
    let composed = TokenFormat::compose(&id, &secret);

    let (parsed_id, parsed_secret) = TokenFormat::parse(&composed).expect("parses");

    assert_eq!(parsed_id, id);
    assert_eq!(parsed_secret, secret);
}

// The alphabet excludes `_` so that `split_once` is unambiguous. A separator inside either
// half would make parsing depend on which occurrence it split at.
#[test]
fn minted_parts_never_contain_the_separator() {
    for _ in 0..64 {
        let (id, secret) = TokenFormat::mint();
        assert!(id.chars().all(|c| c.is_ascii_alphanumeric()), "id: {id}");
        assert!(
            secret.chars().all(|c| c.is_ascii_alphanumeric()),
            "secret: {secret}"
        );
    }
}

#[test]
fn a_value_without_the_prefix_does_not_parse() {
    let (id, secret) = TokenFormat::mint();
    let bare = format!("{id}_{secret}");

    assert!(TokenFormat::parse(&bare).is_none());
}

// A beta.21 deployment's scalar must fall through to the legacy branch, not be mistaken
// for a malformed identified token.
#[test]
fn a_legacy_scalar_does_not_parse() {
    assert!(TokenFormat::parse("aB3xY7qLmN2pR8sT4vW6zC9dF1gH5jK0").is_none());
}

#[test]
fn wrong_lengths_do_not_parse() {
    assert!(TokenFormat::parse("bvc_short_abc").is_none());
    assert!(TokenFormat::parse("bvc__").is_none());
    assert!(TokenFormat::parse("bvc_").is_none());
    assert!(TokenFormat::parse("").is_none());
}

// Two mints must not collide, or a revocation would retire someone else's credential.
#[test]
fn ids_differ_between_mints() {
    let (first, _) = TokenFormat::mint();
    let (second, _) = TokenFormat::mint();

    assert_ne!(first, second);
}

#[test]
fn the_hash_is_stable_and_hides_the_secret() {
    let hash = TokenFormat::hash("abc");

    assert_eq!(hash, TokenFormat::hash("abc"));
    assert_ne!(hash, TokenFormat::hash("abd"));
    assert_eq!(hash.len(), 64);
    assert!(!hash.contains("abc"));
}
