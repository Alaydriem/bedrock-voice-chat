use std::collections::HashMap;

use bvc_server_lib::config::{ApplicationConfig, PlayTls, Server};

fn with_manual_certificate() -> Server {
    let mut server = Server::default();
    server.tls.certificate = "/etc/bvc/cert.pem".to_string();
    server.tls.key = "/etc/bvc/key.pem".to_string();
    server
}

// A manual certificate is issued for names only the operator knows. Starting without them
// serves the certificate under a name it does not cover, and every consumer of tls.names
// (the CA SANs, the mod's chat endpoint, Meridian) works from the wrong one.
#[test]
fn a_manual_certificate_without_names_is_a_problem() {
    let problem = with_manual_certificate()
        .tls_names_problem()
        .expect("a problem is reported");

    assert!(problem.contains("tls.names"));
    assert!(problem.contains("tls.certificate"));
}

#[test]
fn a_manual_certificate_with_only_blank_names_is_a_problem() {
    let mut config = with_manual_certificate();
    config.tls.names = vec!["".to_string(), "   ".to_string()];

    assert!(config.tls_names_problem().is_some());
}

// Half a pair is still a manual source. The missing half is refused elsewhere; the
// missing names must not hide behind it.
#[test]
fn a_key_alone_without_names_is_a_problem() {
    let mut config = Server::default();
    config.tls.key = "/etc/bvc/key.pem".to_string();

    assert!(config.tls_names_problem().is_some());
}

#[test]
fn a_manual_certificate_with_a_name_is_not_a_problem() {
    let mut config = with_manual_certificate();
    config.tls.names = vec!["voice.example.com".to_string()];

    assert!(config.tls_names_problem().is_none());
}

// Play takes its name from the registry, so an empty list is expected there.
#[test]
fn play_without_names_is_not_a_problem() {
    let mut config = Server::default();
    config.tls.play = Some(PlayTls::default());

    assert!(config.tls_names_problem().is_none());
}

// No source at all is refused by `get_rocket_config`, which names the missing
// certificate file.
#[test]
fn no_source_at_all_is_not_a_names_problem() {
    assert!(Server::default().tls_names_problem().is_none());
}

// The embedded mods hand the server JSON with a certificate pair and no names. A default
// name filled in during parsing would satisfy the check and let that config start.
#[test]
fn an_embedded_config_omitting_names_is_a_problem() {
    let json = r#"{
        "server": {
            "port": 28280,
            "tls": { "certificate": "/etc/bvc/cert.pem", "key": "/etc/bvc/key.pem" }
        }
    }"#;

    let config = ApplicationConfig::from_json_with_env(json, HashMap::new())
        .expect("the config parses");

    assert!(config.server.tls_names_problem().is_some());
}
