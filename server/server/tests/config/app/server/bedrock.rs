use bvc_server_lib::config::{BedrockConfig, BedrockServerEntry};
use common::structs::bedrock::AddonMode;

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
            { name = "The Hive", host = "geo.hivebedrock.network", addon_mode = "no_net" },
            { name = "Custom", host = "play.example.com", port = 25000, protocol_version = 844, addon_mode = "net" },
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
            { name = "The Hive", host = "geo.hivebedrock.network", addon_mode = "no_net" },
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
fn compact_form_without_optional_tokens_defaults_to_net() {
    let entry = BedrockServerEntry::from_compact("Survival@play.example.com")
        .expect("two-segment form must still parse");
    assert_eq!(entry.host, "play.example.com");
    assert_eq!(entry.port, 19132);
    assert_eq!(entry.protocol_version, None);
    assert_eq!(entry.addon_mode, AddonMode::Net);
}

#[test]
fn compact_form_reads_a_mode_token_without_a_protocol() {
    let entry = BedrockServerEntry::from_compact("Survival@play.example.com@net")
        .expect("mode token alone must parse");
    assert_eq!(entry.protocol_version, None);
    assert_eq!(entry.addon_mode, AddonMode::Net);
}

#[test]
fn compact_form_reads_both_optional_tokens_in_either_order() {
    let protocol_first =
        BedrockServerEntry::from_compact("S@h:25000@844@net").expect("protocol then mode");
    assert_eq!(protocol_first.port, 25000);
    assert_eq!(protocol_first.protocol_version, Some(844));
    assert_eq!(protocol_first.addon_mode, AddonMode::Net);

    let mode_first =
        BedrockServerEntry::from_compact("S@h:25000@net@844").expect("mode then protocol");
    assert_eq!(mode_first.protocol_version, Some(844));
    assert_eq!(mode_first.addon_mode, AddonMode::Net);
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

// An advertised server whose mode nobody declared is an operator mistake, not a
// value to guess at: guessing wrong either duplicates every event the addon
// already delivers or stops delivering them at all.
#[test]
fn an_entry_without_a_mode_is_rejected() {
    let hcl = r#"
        enabled = true
        servers = [
            { name = "Ambiguous", host = "play.example.com" },
        ]
    "#;
    let value: serde_json::Value = hcl::from_str(hcl).expect("parse hcl");
    let result: Result<BedrockConfig, _> = serde_json::from_value(value);
    assert!(
        result.is_err(),
        "an advertised server with no declared mode must not deserialize"
    );
}

// Three ports live in this area and two of them are easy to conflate. Asserting
// them together is what stops a future find-and-replace on 19132 from silently
// repointing every advertised server at the local proxy.
#[test]
fn the_three_bedrock_ports_are_distinct_and_stable() {
    let config = BedrockConfig::default();
    assert_eq!(config.transfer_port, 28283, "the relay listens on 28283");
    assert_eq!(
        config.transfer_target_port,
        common::consts::bedrock::BEDROCK_LISTEN_PORT,
        "the relay must transfer to whatever port the client actually listens on"
    );
    assert_eq!(
        common::consts::bedrock::BEDROCK_LISTEN_PORT,
        28282,
        "the client proxy listens on 28282"
    );

    let entry =
        BedrockServerEntry::from_compact("Upstream@play.example.com@net").expect("parse");
    assert_eq!(
        entry.port, 19132,
        "an advertised entry still defaults to the real BDS port"
    );
}

// An operator upgrading past the DNS removal still has a `dns` block in their
// config. Ignoring it keeps the server booting; rejecting it would turn a feature
// that quietly stopped existing into a server that will not start.
#[test]
fn an_old_config_with_a_dns_block_still_loads() {
    let hcl = r#"
        enabled = true
        dns {
            enabled = true
            override_host = "geo.hivebedrock.network"
        }
        servers = []
    "#;
    let value: serde_json::Value = hcl::from_str(hcl).expect("parse hcl");
    let config: BedrockConfig =
        serde_json::from_value(value).expect("a leftover dns block must not break startup");
    assert!(config.enabled);
    assert!(config.servers.is_empty());
}

#[test]
fn hcl_accepts_an_explicit_no_net_declaration() {
    let hcl = r#"
        enabled = true
        servers = [
            { name = "Aternos", host = "smp.aternos.me", addon_mode = "no_net" },
        ]
    "#;
    let value: serde_json::Value = hcl::from_str(hcl).expect("parse hcl");
    let config: BedrockConfig = serde_json::from_value(value).expect("deserialize bedrock config");

    assert_eq!(
        config.to_api().servers[0].addon_mode,
        AddonMode::NoNet
    );
}

#[test]
fn hcl_mode_reaches_the_api_view() {
    let hcl = r#"
        enabled = true
        servers = [
            { name = "Survival", host = "play.example.com", addon_mode = "net" },
            { name = "Legacy", host = "old.example.com", addon_mode = "no_net" },
        ]
    "#;
    let value: serde_json::Value = hcl::from_str(hcl).expect("parse hcl");
    let config: BedrockConfig = serde_json::from_value(value).expect("deserialize bedrock config");

    let api = config.to_api();
    assert_eq!(api.servers[0].addon_mode, AddonMode::Net);
    assert_eq!(
        api.servers[1].addon_mode,
        AddonMode::NoNet,
        "each entry carries its own declaration"
    );
}
