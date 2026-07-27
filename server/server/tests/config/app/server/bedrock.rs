use bvc_server_lib::config::BedrockConfig;

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
