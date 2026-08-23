//! What a QUIC demotion means once it has been recorded.
//!
//! The behaviour under test is not "can a flag be set" but the two properties the field
//! failure depends on: the verdict outlives the reconnect that produced it, and the two
//! callers that write and read it agree on the key despite holding the server's name in
//! different forms.

use bvc_client_lib::network::TransportVerdict;

#[test]
fn a_demotion_survives_every_later_attempt() {
    let verdict = TransportVerdict::new();
    assert!(!verdict.is_demoted("voice.example.com"));

    verdict.demote("voice.example.com");

    // Re-read many times over: nothing expires it, and nothing about asking clears it.
    // A TTL here would return a player to a transport already shown to break, and the
    // failure that follows is silence rather than an error.
    for _ in 0..1_000 {
        assert!(
            verdict.is_demoted("voice.example.com"),
            "a demotion must hold for the rest of the run"
        );
    }
}

#[test]
fn a_demotion_applies_only_to_the_server_that_earned_it() {
    let verdict = TransportVerdict::new();
    verdict.demote("broken.example.com");

    assert!(verdict.is_demoted("broken.example.com"));
    assert!(
        !verdict.is_demoted("healthy.example.com"),
        "a different server must be judged on its own evidence"
    );
}

/// Selection knows the bare FQDN; the health monitor knows the URL it was told to probe.
/// If those did not normalise to one key the demotion would be written under one spelling
/// and read under another, which looks exactly like the demotion never happening.
#[test]
fn the_url_and_the_hostname_are_the_same_verdict() {
    let verdict = TransportVerdict::new();
    verdict.demote("https://voice.example.com:443/api");

    assert!(verdict.is_demoted("voice.example.com"));
}

#[test]
fn a_hostname_demotion_is_seen_through_a_url() {
    let verdict = TransportVerdict::new();
    verdict.demote("Voice.Example.COM");

    assert!(
        verdict.is_demoted("https://voice.example.com:8443/"),
        "case and port must not split one server into two verdicts"
    );
}

/// An IPv6 literal is bracketed, so the last colon separates a port only outside the
/// brackets. Splitting on it naively would key a demotion on a fragment of the address.
#[test]
fn an_ipv6_literal_keeps_its_address() {
    let verdict = TransportVerdict::new();
    verdict.demote("https://[2001:db8::1]:443");

    assert!(verdict.is_demoted("[2001:db8::1]"));
    assert!(!verdict.is_demoted("2001:db8"));
}
