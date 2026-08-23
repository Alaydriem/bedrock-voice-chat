use bvc_client_lib::websocket::WebSocketConfig;

// A stored config written before `allow_external` existed must still deserialize, and must
// not read as permitting external access.
#[test]
fn a_config_without_allow_external_defaults_to_refusing_it() {
    let stored = r#"{"enabled":true,"localhost_only":true,"port":9595,"key":"abc"}"#;
    let config: WebSocketConfig = serde_json::from_str(stored).expect("older config deserializes");
    assert!(!config.allow_external);
    assert_eq!(config.port, 9595);
    assert_eq!(config.key, "abc");
}

// The bind address follows `allow_external` alone. `localhost_only` is retained only so the
// settings manager can run its one-time migration, and must no longer influence the bind.
#[test]
fn bind_host_follows_allow_external_and_ignores_localhost_only() {
    let mut config = WebSocketConfig::default();
    config.localhost_only = false;
    assert_eq!(config.bind_host(), "127.0.0.1");

    config.allow_external = true;
    assert_eq!(config.bind_host(), "0.0.0.0");
}
