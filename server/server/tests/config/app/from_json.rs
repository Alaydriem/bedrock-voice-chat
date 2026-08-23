use std::collections::HashMap;

use bvc_server_lib::config::ApplicationConfig;

fn minimal_json() -> &'static str {
    r#"{"server":{"port":8444,"quic_port":8443}}"#
}

#[test]
fn parses_json_and_applies_serde_defaults() {
    let config = ApplicationConfig::from_json_with_env(minimal_json(), HashMap::new())
        .expect("minimal config parses");

    assert_eq!(config.server.port, 8444);
    assert_eq!(config.log.level, "info");
    assert_eq!(config.database.scheme, "sqlite3");
}

// The embedded server is configured by a file the mod writes, but a panel or
// container host sets variables. Precedence must match the CLI path.
#[test]
fn an_env_override_beats_the_json_value() {
    let vars = HashMap::from([("BVC_QUIC_PORT".to_string(), "9443".to_string())]);
    let config =
        ApplicationConfig::from_json_with_env(minimal_json(), vars).expect("config parses");

    assert_eq!(config.server.quic_port, 9443);
    assert_eq!(config.server.port, 8444);
}

#[test]
fn a_malformed_env_override_is_an_error_naming_the_variable() {
    let vars = HashMap::from([("BVC_QUIC_PORT".to_string(), "not-a-port".to_string())]);
    let error = ApplicationConfig::from_json_with_env(minimal_json(), vars)
        .err()
        .expect("a malformed override fails");

    assert!(
        error.to_string().contains("BVC_QUIC_PORT"),
        "the error must name the variable, got: {error}"
    );
}

#[test]
fn invalid_json_is_an_error() {
    let error = ApplicationConfig::from_json_with_env("{ not json", HashMap::new())
        .err()
        .expect("invalid JSON fails");

    assert!(!error.to_string().is_empty());
}

// The mod reads the resolved config to build its chat endpoint, so what the
// getter returns must round-trip back into the same configuration.
#[test]
fn the_resolved_config_serializes_back_to_json() {
    let vars = HashMap::from([("BVC_QUIC_PORT".to_string(), "9443".to_string())]);
    let config =
        ApplicationConfig::from_json_with_env(minimal_json(), vars).expect("config parses");

    let rendered = serde_json::to_string(&config).expect("resolved config serializes");
    let reparsed = ApplicationConfig::from_json_with_env(&rendered, HashMap::new())
        .expect("resolved config reparses");

    assert_eq!(reparsed.server.quic_port, 9443);
    assert_eq!(reparsed.server.port, 8444);
    assert_eq!(reparsed.log.level, "info");
}
