use bvc_relay_service::acme::CloudflareDns;

// The zone is discovered from the hostname by walking its labels, so one token with
// access to both zones needs no second zone id in config. A configured id that
// pointed at the wrong zone would fail at issuance with a permissions error rather
// than anything naming the cause.
#[test]
fn zone_candidates_walk_the_domain_from_most_specific_to_least() {
    let candidates = CloudflareDns::zone_candidates("registry.bedrockvoicechat.com");

    assert_eq!(
        candidates,
        vec![
            "registry.bedrockvoicechat.com".to_string(),
            "bedrockvoicechat.com".to_string(),
        ]
    );
}

// A bare apex is its own only candidate. Walking past it would query the public
// suffix, which no account owns.
#[test]
fn an_apex_domain_is_its_own_only_candidate() {
    assert_eq!(
        CloudflareDns::zone_candidates("bedrockvc.stream"),
        vec!["bedrockvc.stream".to_string()]
    );
}

// The challenge record goes beneath the `_acme-challenge` label, which is where the
// certificate authority looks and nowhere else.
#[test]
fn the_challenge_record_name_is_prefixed() {
    assert_eq!(
        CloudflareDns::challenge_name("registry.bedrockvoicechat.com"),
        "_acme-challenge.registry.bedrockvoicechat.com"
    );
}
