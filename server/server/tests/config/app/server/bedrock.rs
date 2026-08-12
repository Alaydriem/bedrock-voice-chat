use bvc_server_lib::config::{BedrockConfig, BedrockServerEntry};
use common::structs::bedrock::AddonTransport;

#[test]
fn servers_default_to_empty_when_absent() {
    let config: BedrockConfig =
        serde_json::from_str("{}").expect("deserialize empty bedrock block");
    assert!(config.servers.is_empty());
    assert!(config.to_api().servers.is_empty());
}

#[test]
fn hcl_servers_reach_the_api_view_with_defaults_applied() {
    let hcl = r#"
        enabled = true
        servers = [
            { name = "The Hive", host = "geo.hivebedrock.network" },
            { name = "Custom", host = "play.example.com", port = 25000, protocol_version = 844 },
        ]
    "#;
    let value: serde_json::Value = hcl::from_str(hcl).expect("parse hcl");
    let config: BedrockConfig = serde_json::from_value(value).expect("deserialize bedrock config");

    let api = config.to_api();
    assert_eq!(api.servers.len(), 2);
    assert_eq!(api.servers[0].name, "The Hive");
    assert_eq!(api.servers[0].host, "geo.hivebedrock.network");
    // Omitted port falls back to the Bedrock default.
    assert_eq!(api.servers[0].port, 19132);
    assert_eq!(api.servers[0].protocol_version, None);
    assert_eq!(api.servers[1].port, 25000);
    assert_eq!(api.servers[1].protocol_version, Some(844));
}

#[test]
fn disabled_relay_withholds_ports_and_servers() {
    let hcl = r#"
        enabled = false
        servers = [
            { name = "The Hive", host = "geo.hivebedrock.network" },
        ]
    "#;
    let value: serde_json::Value = hcl::from_str(hcl).expect("parse hcl");
    let config: BedrockConfig = serde_json::from_value(value).expect("deserialize bedrock config");

    let api = config.to_api();
    assert!(!api.enabled);
    assert_eq!(api.transfer_port, None);
    assert!(
        api.servers.is_empty(),
        "a disabled relay has nothing a client can connect to"
    );
}

#[test]
fn compact_form_without_optional_tokens_defaults_to_no_net() {
    let entry = BedrockServerEntry::from_compact("Survival@play.example.com")
        .expect("two-segment form must still parse");
    assert_eq!(entry.host, "play.example.com");
    assert_eq!(entry.port, 19132);
    assert_eq!(entry.protocol_version, None);
    assert_eq!(entry.addon_transport, AddonTransport::NoNet);
}

#[test]
fn compact_form_reads_a_transport_token_without_a_protocol() {
    let entry = BedrockServerEntry::from_compact("Survival@play.example.com@net")
        .expect("transport token alone must parse");
    assert_eq!(entry.protocol_version, None);
    assert_eq!(entry.addon_transport, AddonTransport::Net);
}

#[test]
fn compact_form_reads_both_optional_tokens_in_either_order() {
    let protocol_first =
        BedrockServerEntry::from_compact("S@h:25000@844@net").expect("protocol then transport");
    assert_eq!(protocol_first.port, 25000);
    assert_eq!(protocol_first.protocol_version, Some(844));
    assert_eq!(protocol_first.addon_transport, AddonTransport::Net);

    let transport_first =
        BedrockServerEntry::from_compact("S@h:25000@net@844").expect("transport then protocol");
    assert_eq!(transport_first.protocol_version, Some(844));
    assert_eq!(transport_first.addon_transport, AddonTransport::Net);
}

#[test]
fn compact_form_names_an_unrecognized_token_in_its_error() {
    let err = BedrockServerEntry::from_compact("S@h@banana")
        .expect_err("an unknown token must not be silently ignored");
    assert!(
        err.to_string().contains("banana"),
        "error must name the offending token, got: {err}"
    );
}

#[test]
fn hcl_transport_reaches_the_api_view() {
    let hcl = r#"
        enabled = true
        servers = [
            { name = "Survival", host = "play.example.com", addon_transport = "net" },
            { name = "Legacy", host = "old.example.com" },
        ]
    "#;
    let value: serde_json::Value = hcl::from_str(hcl).expect("parse hcl");
    let config: BedrockConfig = serde_json::from_value(value).expect("deserialize bedrock config");

    let api = config.to_api();
    assert_eq!(api.servers[0].addon_transport, AddonTransport::Net);
    assert_eq!(
        api.servers[1].addon_transport,
        AddonTransport::NoNet,
        "an entry that declares nothing must stay no-net"
    );
}
