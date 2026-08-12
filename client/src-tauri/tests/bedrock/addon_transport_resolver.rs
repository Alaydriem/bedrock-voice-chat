use bvc_client_lib::bedrock::AddonTransportResolver;
use common::response::ApiConfigBedrockServer;
use common::structs::bedrock::AddonTransport;

fn advertised(host: &str, port: u16, transport: AddonTransport) -> ApiConfigBedrockServer {
    ApiConfigBedrockServer {
        name: "Advertised".to_string(),
        host: host.to_string(),
        port,
        protocol_version: None,
        addon_transport: transport,
    }
}

#[test]
fn an_explicit_request_value_wins_over_the_advertised_list() {
    let list = vec![advertised("play.example.com", 19132, AddonTransport::Net)];
    let resolved = AddonTransportResolver::proxy(
        Some(AddonTransport::NoNet),
        &list,
        "play.example.com",
        19132,
    );
    assert_eq!(resolved, AddonTransport::NoNet);
}

#[test]
fn a_custom_entry_inherits_from_a_matching_advertised_server() {
    let list = vec![advertised("play.example.com", 19132, AddonTransport::Net)];
    let resolved = AddonTransportResolver::proxy(None, &list, "play.example.com", 19132);
    assert_eq!(resolved, AddonTransport::Net);
}

#[test]
fn host_matching_ignores_case() {
    let list = vec![advertised("Play.Example.Com", 19132, AddonTransport::Net)];
    let resolved = AddonTransportResolver::proxy(None, &list, "play.example.com", 19132);
    assert_eq!(resolved, AddonTransport::Net);
}

#[test]
fn a_port_mismatch_is_not_a_match() {
    let list = vec![advertised("play.example.com", 19132, AddonTransport::Net)];
    let resolved = AddonTransportResolver::proxy(None, &list, "play.example.com", 25000);
    assert_eq!(resolved, AddonTransport::NoNet);
}

#[test]
fn an_unknown_target_falls_back_to_no_net() {
    let resolved = AddonTransportResolver::proxy(None, &[], "unknown.example.com", 19132);
    assert_eq!(resolved, AddonTransport::NoNet);
}
