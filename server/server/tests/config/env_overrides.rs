use std::collections::HashMap;

use bvc_server_lib::config::{AcmeProviderKind, ApplicationConfig, EnvOverrides};

fn vars(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn apply(pairs: &[(&str, &str)], config: ApplicationConfig) -> ApplicationConfig {
    EnvOverrides::from_vars(vars(pairs))
        .apply(config)
        .expect("apply must succeed")
}

#[test]
fn override_beats_config_value() {
    let mut config = ApplicationConfig::default();
    config.server.minecraft.access_token = "from-config".to_string();
    let config = apply(&[("BVC_ACCESS_TOKEN", "from-env")], config);
    assert_eq!(config.server.minecraft.access_token, "from-env");
}

#[test]
fn unset_and_empty_vars_leave_config_untouched() {
    let mut config = ApplicationConfig::default();
    config.server.minecraft.access_token = "keep-me".to_string();
    config.server.quic_port = 8443;
    let config = apply(&[("BVC_ACCESS_TOKEN", "")], config);
    assert_eq!(config.server.minecraft.access_token, "keep-me");
    assert_eq!(config.server.quic_port, 8443);
}

#[test]
fn bvc_server_sets_listen_and_port_but_ignores_urls() {
    let config = apply(&[("BVC_SERVER", "0.0.0.0:8444")], ApplicationConfig::default());
    assert_eq!(config.server.listen, "0.0.0.0");
    assert_eq!(config.server.port, 8444);

    let config = apply(
        &[("BVC_SERVER", "https://127.0.0.1:3000")],
        ApplicationConfig::default(),
    );
    assert_eq!(config.server.listen, ApplicationConfig::default().server.listen);
    assert_eq!(config.server.port, ApplicationConfig::default().server.port);
}

#[test]
fn advertised_quic_ports_parses_a_list_and_preserves_order() {
    let config = apply(
        &[("BVC_ADVERTISED_QUIC_PORTS", "443, 8443")],
        ApplicationConfig::default(),
    );
    assert_eq!(config.server.advertised_quic_ports, vec![443u32, 8443]);
    assert_eq!(
        config.server.quic_ports(),
        vec![443u32, 8443],
        "the override must reach what clients are actually told"
    );
}

#[test]
fn advertised_quic_ports_rejects_a_malformed_entry() {
    let err = EnvOverrides::from_vars(vars(&[("BVC_ADVERTISED_QUIC_PORTS", "443,not-a-port")]))
        .apply(ApplicationConfig::default())
        .expect_err("a malformed port must be a hard startup error, not a silently dropped entry");
    assert!(format!("{err}").contains("BVC_ADVERTISED_QUIC_PORTS"));
}

#[test]
fn an_empty_advertised_quic_ports_variable_leaves_the_default_alone() {
    let mut config = ApplicationConfig::default();
    config.server.quic_port = 8443;
    let config = apply(&[("BVC_ADVERTISED_QUIC_PORTS", "")], config);
    assert_eq!(
        config.server.quic_ports(),
        vec![443u32, 28280],
        "an empty variable in a compose file must not blank out the advertisement"
    );
}

#[test]
fn quic_port_parses_or_errors() {
    let config = apply(&[("BVC_QUIC_PORT", "8443")], ApplicationConfig::default());
    assert_eq!(config.server.quic_port, 8443);

    let err = EnvOverrides::from_vars(vars(&[("BVC_QUIC_PORT", "not-a-port")]))
        .apply(ApplicationConfig::default())
        .expect_err("malformed port must error");
    assert!(format!("{err}").contains("BVC_QUIC_PORT"));
}

#[test]
fn tls_names_and_ips_split_on_commas_and_trim() {
    let config = apply(
        &[
            ("BVC_TLS_NAMES", "a.example, b.example ,,c.example"),
            ("BVC_TLS_IPS", "10.0.0.1, 10.0.0.2"),
        ],
        ApplicationConfig::default(),
    );
    assert_eq!(config.server.tls.names, vec!["a.example", "b.example", "c.example"]);
    assert_eq!(config.server.tls.ips, vec!["10.0.0.1", "10.0.0.2"]);
}

#[test]
fn tls_paths_override() {
    let config = apply(
        &[
            ("BVC_TLS_CERTIFICATE", "/certs/fullchain.pem"),
            ("BVC_TLS_KEY", "/certs/privkey.pem"),
            ("BVC_TLS_CERTS_PATH", "/data/certificates"),
        ],
        ApplicationConfig::default(),
    );
    assert_eq!(config.server.tls.certificate, "/certs/fullchain.pem");
    assert_eq!(config.server.tls.key, "/certs/privkey.pem");
    assert_eq!(config.server.tls.certs_path, "/data/certificates");
}

#[test]
fn telemetry_parses_bool_or_errors() {
    let config = apply(&[("BVC_TELEMETRY", "false")], ApplicationConfig::default());
    assert!(!config.server.features.telemetry);

    let config = apply(&[("BVC_TELEMETRY", "TRUE")], ApplicationConfig::default());
    assert!(config.server.features.telemetry);

    let err = EnvOverrides::from_vars(vars(&[("BVC_TELEMETRY", "yes")]))
        .apply(ApplicationConfig::default())
        .expect_err("non-bool telemetry must error");
    assert!(format!("{err}").contains("BVC_TELEMETRY"));
}

#[test]
fn chat_parses_bool_or_errors() {
    let config = apply(&[("BVC_CHAT", "false")], ApplicationConfig::default());
    assert!(!config.server.features.chat);

    let config = apply(&[("BVC_CHAT", "TRUE")], ApplicationConfig::default());
    assert!(config.server.features.chat);

    let err = EnvOverrides::from_vars(vars(&[("BVC_CHAT", "yes")]))
        .apply(ApplicationConfig::default())
        .expect_err("non-bool chat must error");
    assert!(format!("{err}").contains("BVC_CHAT"));
}

#[test]
fn database_fields_override() {
    let config = apply(
        &[
            ("BVC_DATABASE_SCHEME", "mysql"),
            ("BVC_DATABASE_DATABASE", "bvc"),
            ("BVC_DATABASE_HOST", "db.internal"),
            ("BVC_DATABASE_PORT", "3306"),
            ("BVC_DATABASE_USERNAME", "bvc"),
            ("BVC_DATABASE_PASSWORD", "hunter2"),
        ],
        ApplicationConfig::default(),
    );
    assert_eq!(config.database.scheme, "mysql");
    assert_eq!(config.database.database, "bvc");
    assert_eq!(config.database.host.as_deref(), Some("db.internal"));
    assert_eq!(config.database.port, Some(3306));
    assert_eq!(config.database.username.as_deref(), Some("bvc"));
    assert_eq!(config.database.password.as_deref(), Some("hunter2"));
}

#[test]
fn database_ssl_fields_override() {
    let config = apply(
        &[
            ("BVC_DATABASE_SCHEME", "postgres"),
            ("BVC_DATABASE_SSL_MODE", "verify-full"),
            ("BVC_DATABASE_SSL_ROOT_CERT", "/certs/ca.pem"),
            ("BVC_DATABASE_SSL_CERT", "/certs/client.pem"),
            ("BVC_DATABASE_SSL_KEY", "/certs/client.key"),
        ],
        ApplicationConfig::default(),
    );
    assert_eq!(config.database.ssl_mode.as_deref(), Some("verify-full"));
    assert_eq!(
        config.database.ssl_root_cert.as_deref(),
        Some("/certs/ca.pem")
    );
    assert_eq!(config.database.ssl_cert.as_deref(), Some("/certs/client.pem"));
    assert_eq!(config.database.ssl_key.as_deref(), Some("/certs/client.key"));
}

#[test]
fn meridian_full_set_materializes_block() {
    let config = apply(
        &[
            ("BVC_MERIDIAN_URL", "https://meridian.internal:9443"),
            ("BVC_MERIDIAN_API_KEY", "key"),
            ("BVC_MERIDIAN_INSTANCE_ID", "7"),
            ("BVC_MERIDIAN_NAME", "customer-x"),
            ("BVC_MERIDIAN_BACKEND", "10.0.1.5"),
        ],
        ApplicationConfig::default(),
    );
    let meridian = config.server.meridian.expect("block must materialize");
    assert_eq!(meridian.url, "https://meridian.internal:9443");
    assert_eq!(meridian.api_key, "key");
    assert_eq!(meridian.instance_id, 7);
    assert_eq!(meridian.name, "customer-x");
    assert_eq!(meridian.backend, "10.0.1.5");
    assert_eq!(meridian.host, None);
}

#[test]
fn meridian_partial_set_errors_listing_missing_vars() {
    let err = EnvOverrides::from_vars(vars(&[("BVC_MERIDIAN_URL", "https://m:9443")]))
        .apply(ApplicationConfig::default())
        .expect_err("partial meridian set must error");
    let msg = format!("{err}");
    assert!(msg.contains("BVC_MERIDIAN_API_KEY"), "got: {msg}");
    assert!(msg.contains("BVC_MERIDIAN_NAME"), "got: {msg}");
}

#[test]
fn acme_vars_materialize_block() {
    let config = apply(
        &[
            ("BVC_ACME_EMAIL", "ops@example.com"),
            ("BVC_ACME_PROVIDER", "cloudflare"),
            ("BVC_ACME_API_TOKEN", "cf-token"),
            ("BVC_ACME_DOMAINS", "a.example.com, b.example.com"),
        ],
        ApplicationConfig::default(),
    );
    let acme = config.server.tls.acme.expect("block must materialize");
    assert_eq!(acme.email, "ops@example.com");
    assert_eq!(acme.provider, Some(AcmeProviderKind::Cloudflare));
    assert_eq!(acme.api_token.as_deref(), Some("cf-token"));
    assert_eq!(
        acme.domains,
        Some(vec!["a.example.com".to_string(), "b.example.com".to_string()])
    );
    assert_eq!(acme.directory, "https://acme-v02.api.letsencrypt.org/directory");
}

#[test]
fn acme_partial_set_errors_listing_missing_vars() {
    let err = EnvOverrides::from_vars(vars(&[("BVC_ACME_API_TOKEN", "cf-token")]))
        .apply(ApplicationConfig::default())
        .expect_err("acme without email+provider must error");
    let msg = format!("{err}");
    assert!(msg.contains("BVC_ACME_EMAIL"), "got: {msg}");
    assert!(msg.contains("BVC_ACME_PROVIDER"), "got: {msg}");
}

#[test]
fn acme_vars_override_existing_block_fields() {
    let mut config = ApplicationConfig::default();
    let mut acme = bvc_server_lib::config::Acme::default();
    acme.email = "old@example.com".to_string();
    acme.provider = Some(AcmeProviderKind::Cloudflare);
    config.server.tls.acme = Some(acme);
    let config = apply(
        &[
            ("BVC_ACME_DNS_URL", "https://acme-dns.internal"),
            ("BVC_ACME_PROVIDER", "acme-dns"),
        ],
        config,
    );
    let acme = config.server.tls.acme.expect("block still present");
    assert_eq!(acme.provider, Some(AcmeProviderKind::AcmeDns));
    assert_eq!(acme.server_url.as_deref(), Some("https://acme-dns.internal"));
    assert_eq!(acme.email, "old@example.com");
}

#[test]
fn acme_provider_env_rejects_unknown_value() {
    let err = EnvOverrides::from_vars(vars(&[
        ("BVC_ACME_EMAIL", "ops@example.com"),
        ("BVC_ACME_PROVIDER", "route53"),
    ]))
    .apply(ApplicationConfig::default())
    .expect_err("unknown provider must error");
    let msg = format!("{err}");
    assert!(msg.contains("BVC_ACME_PROVIDER"), "got: {msg}");
    assert!(msg.contains("route53"), "got: {msg}");
}

#[test]
fn meridian_vars_override_existing_block_fields() {
    let mut config = ApplicationConfig::default();
    config.server.meridian = Some(bvc_server_lib::config::Meridian {
        url: "https://old:9443".to_string(),
        api_key: "old-key".to_string(),
        instance_id: 1,
        name: "old-name".to_string(),
        host: None,
        backend: "10.0.0.1".to_string(),
    });
    let config = apply(&[("BVC_MERIDIAN_API_KEY", "new-key")], config);
    let meridian = config.server.meridian.expect("block still present");
    assert_eq!(meridian.api_key, "new-key");
    assert_eq!(meridian.url, "https://old:9443");
}

#[test]
fn bedrock_enabled_parses_bool_or_errors() {
    let config = apply(&[("BVC_BEDROCK_ENABLED", "false")], ApplicationConfig::default());
    assert!(!config.server.bedrock.enabled);

    let config = apply(&[("BVC_BEDROCK_ENABLED", "TRUE")], ApplicationConfig::default());
    assert!(config.server.bedrock.enabled);

    let err = EnvOverrides::from_vars(vars(&[("BVC_BEDROCK_ENABLED", "yes")]))
        .apply(ApplicationConfig::default())
        .unwrap_err();
    assert!(format!("{err}").contains("BVC_BEDROCK_ENABLED"));
}

#[test]
fn bedrock_transfer_port_parses_u16_or_errors() {
    let config = apply(
        &[("BVC_BEDROCK_TRANSFER_PORT", "19140")],
        ApplicationConfig::default(),
    );
    assert_eq!(config.server.bedrock.transfer_port, 19140);

    let err = EnvOverrides::from_vars(vars(&[("BVC_BEDROCK_TRANSFER_PORT", "70000")]))
        .apply(ApplicationConfig::default())
        .unwrap_err();
    assert!(format!("{err}").contains("BVC_BEDROCK_TRANSFER_PORT"));
}

#[test]
fn bedrock_servers_parse_compact_syntax_and_replace_config_list() {
    let mut config = ApplicationConfig::default();
    config.server.bedrock.servers = vec![bvc_server_lib::config::BedrockServerEntry {
        name: "From Config".to_string(),
        host: "config.example.com".to_string(),
        port: 19132,
        protocol_version: None,
        addon_mode: Default::default(),
    }];

    let config = apply(
        &[(
            "BVC_BEDROCK_SERVERS",
            "The Hive@geo.hivebedrock.network, Custom@play.example.com:25000@844",
        )],
        config,
    );

    let servers = &config.server.bedrock.servers;
    assert_eq!(servers.len(), 2, "env list replaces the config list");
    assert_eq!(servers[0].name, "The Hive");
    assert_eq!(servers[0].host, "geo.hivebedrock.network");
    assert_eq!(servers[0].port, 19132);
    assert_eq!(servers[0].protocol_version, None);
    assert_eq!(servers[1].name, "Custom");
    assert_eq!(servers[1].host, "play.example.com");
    assert_eq!(servers[1].port, 25000);
    assert_eq!(servers[1].protocol_version, Some(844));
}

#[test]
fn bedrock_servers_malformed_entry_is_a_hard_error() {
    let err = EnvOverrides::from_vars(vars(&[("BVC_BEDROCK_SERVERS", "no-at-sign")]))
        .apply(ApplicationConfig::default())
        .unwrap_err();
    assert!(format!("{err}").contains("BVC_BEDROCK_SERVERS"));

    let err = EnvOverrides::from_vars(vars(&[("BVC_BEDROCK_SERVERS", "Name@host:notaport")]))
        .apply(ApplicationConfig::default())
        .unwrap_err();
    assert!(format!("{err}").contains("BVC_BEDROCK_SERVERS"));
}

#[test]
fn bvc_recording_false_disables_an_enabled_config() {
    let config = ApplicationConfig::default();
    assert!(
        config.voice.recording.enabled,
        "precondition: the default permits recording"
    );

    let config = apply(&[("BVC_RECORDING", "false")], config);

    assert!(!config.voice.recording.enabled);
}

#[test]
fn bvc_recording_absent_leaves_the_config_value() {
    let mut config = ApplicationConfig::default();
    config.voice.recording.enabled = false;

    let config = apply(&[], config);

    assert!(
        !config.voice.recording.enabled,
        "an unset variable must never resurrect a value the operator turned off"
    );
}

#[test]
fn bvc_recording_rejects_a_non_boolean() {
    let err = EnvOverrides::from_vars(vars(&[("BVC_RECORDING", "sometimes")]))
        .apply(ApplicationConfig::default())
        .expect_err("a malformed boolean must be a hard startup error");
    assert!(format!("{err}").contains("BVC_RECORDING"));
}
