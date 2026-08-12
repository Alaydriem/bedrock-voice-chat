use bvc_client_lib::bedrock::AddonModeResolver;
use common::response::ApiConfigBedrockServer;
use common::structs::bedrock::AddonMode;

fn advertised(host: &str, port: u16, transport: AddonMode) -> ApiConfigBedrockServer {
    ApiConfigBedrockServer {
        name: "Advertised".to_string(),
        host: host.to_string(),
        port,
        protocol_version: None,
        addon_mode: transport,
    }
}

#[test]
fn an_explicit_request_value_wins_over_the_advertised_list() {
    let list = vec![advertised("play.example.com", 19132, AddonMode::Net)];
    let resolved = AddonModeResolver::proxy(
        Some(AddonMode::NoNet),
        &list,
        "play.example.com",
        19132,
    );
    assert_eq!(resolved, AddonMode::NoNet);
}

#[test]
fn a_custom_entry_inherits_from_a_matching_advertised_server() {
    let list = vec![advertised("play.example.com", 19132, AddonMode::Net)];
    let resolved = AddonModeResolver::proxy(None, &list, "play.example.com", 19132);
    assert_eq!(resolved, AddonMode::Net);
}

#[test]
fn host_matching_ignores_case() {
    let list = vec![advertised("Play.Example.Com", 19132, AddonMode::Net)];
    let resolved = AddonModeResolver::proxy(None, &list, "play.example.com", 19132);
    assert_eq!(resolved, AddonMode::Net);
}

#[test]
fn a_port_mismatch_is_not_a_match() {
    let list = vec![advertised("play.example.com", 19132, AddonMode::NoNet)];
    let resolved = AddonModeResolver::proxy(None, &list, "play.example.com", 25000);
    assert_eq!(resolved, AddonMode::Net);
}

#[test]
fn an_unknown_target_falls_back_to_net() {
    let resolved = AddonModeResolver::proxy(None, &[], "unknown.example.com", 19132);
    assert_eq!(resolved, AddonMode::Net);
}

#[test]
fn a_known_no_net_host_resolves_without_an_advertised_entry() {
    let resolved = AddonModeResolver::proxy(None, &[], "smp.aternos.me", 19132);
    assert_eq!(resolved, AddonMode::NoNet);
}

#[test]
fn the_bare_known_host_matches_too() {
    let resolved = AddonModeResolver::proxy(None, &[], "aternos.me", 19132);
    assert_eq!(resolved, AddonMode::NoNet);
}

#[test]
fn known_host_matching_ignores_case() {
    let resolved = AddonModeResolver::proxy(None, &[], "SMP.Aternos.ME", 19132);
    assert_eq!(resolved, AddonMode::NoNet);
}

// A lookalike must not match. Suffix matching on a bare `contains` would treat
// this as Aternos and silently stop processing events for a world that needs it.
#[test]
fn a_lookalike_host_does_not_match() {
    let resolved = AddonModeResolver::proxy(None, &[], "aternos.me.evil.example.com", 19132);
    assert_eq!(resolved, AddonMode::Net);
}

#[test]
fn an_explicit_override_still_beats_a_known_host() {
    let resolved = AddonModeResolver::proxy(Some(AddonMode::Net), &[], "smp.aternos.me", 19132);
    assert_eq!(resolved, AddonMode::Net);
}
