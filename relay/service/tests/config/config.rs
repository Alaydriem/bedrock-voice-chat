use std::collections::HashMap;

use bvc_relay_service::config::RelayConfig;

const INTERPOLATED: &str = r#"
listen = "0.0.0.0:28285"
node_dir = "/var/lib/bvc-relay"
database_url = "sqlite::memory:"
zone = "bedrockvc.stream"

discord {
    guild_id = "1234"
    bot_token = "${env.BVC_RELAY_DISCORD_BOT_TOKEN}"
    client_id = "client-id"
    client_secret = "client-secret"
    qualifying_role_ids = ["role-a"]
}

cloudflare {
    api_token = "${env.BVC_RELAY_CLOUDFLARE_TOKEN}"
    zone_id = "zone-id"
}

"#;

const SAMPLE: &str = r#"
listen = "0.0.0.0:28285"
node_dir = "/var/lib/bvc-relay"
database_url = "sqlite::memory:"
zone = "bedrockvc.stream"

discord {
    guild_id = "1234"
    bot_token = "bot-token"
    client_id = "client-id"
    client_secret = "client-secret"
    qualifying_role_ids = ["role-a", "role-b"]
}

cloudflare {
    api_token = "cf-token"
    zone_id = "zone-id"
}

"#;

// A member holding any configured role qualifies. The set is configuration, not a
// tier model, so this is the whole of the entitlement decision.
#[test]
fn a_member_holding_any_configured_role_qualifies() {
    let config = RelayConfig::from_hcl(SAMPLE).expect("sample config parses");

    assert!(config.discord.qualifies(&["role-b".to_string()]));
}

#[test]
fn a_member_holding_no_configured_role_does_not_qualify() {
    let config = RelayConfig::from_hcl(SAMPLE).expect("sample config parses");

    assert!(!config.discord.qualifies(&["role-z".to_string()]));
}

// An empty configured set must refuse everyone rather than admit everyone. A
// misconfigured relay that hands out names to any guild member is worse than one
// that hands out none.
#[test]
fn an_empty_configured_role_set_refuses_everyone() {
    let config = RelayConfig::from_hcl(
        SAMPLE
            .replace(
                r#"qualifying_role_ids = ["role-a", "role-b"]"#,
                "qualifying_role_ids = []",
            )
            .as_str(),
    )
    .expect("config with an empty role set parses");

    assert!(!config.discord.qualifies(&["role-a".to_string()]));
}

// Secrets arrive through the environment rather than living in the file. The
// Cloudflare token can rewrite every operator's address; the bot token reads guild
// membership. Neither belongs in a document an operator copies between hosts.
#[test]
fn env_references_are_interpolated() {
    let mut env = HashMap::new();
    env.insert(
        "BVC_RELAY_DISCORD_BOT_TOKEN".to_string(),
        "bot-secret".to_string(),
    );
    env.insert(
        "BVC_RELAY_CLOUDFLARE_TOKEN".to_string(),
        "cf-secret".to_string(),
    );

    let config = RelayConfig::from_hcl_with_env(INTERPOLATED, &env).expect("interpolates");

    assert_eq!(config.discord.bot_token, "bot-secret");
    assert_eq!(config.cloudflare.api_token, "cf-secret");
}

// An unset variable is a hard error, never a silent empty string. A relay that
// started with an empty Cloudflare token would authenticate against nothing and
// report it only at the first challenge, hours into a deployment.
#[test]
fn an_unset_env_reference_is_refused() {
    let env = HashMap::new();

    assert!(RelayConfig::from_hcl_with_env(INTERPOLATED, &env).is_err());
}

const WITH_HTTP: &str = r#"
listen = "0.0.0.0:28285"
node_dir = "/var/lib/bvc-relay"
database_url = "sqlite::memory:"
zone = "bedrockvc.stream"

discord {
    guild_id = "1234"
    bot_token = "bot-token"
    client_id = "client-id"
    client_secret = "client-secret"
    qualifying_role_ids = ["role-a"]
}

cloudflare {
    api_token = "cf-token"
    zone_id = "zone-id"
}

http {
    hostname = "registry.bedrockvoicechat.com"
    page_origin = "https://bedrockvoicechat.com"
    cert_dir = "/var/lib/bvc-relay/certs"

    acme "cloudflare" {
        email = "ops@example.com"
        api_token = "cf-multi-zone-token"
    }
}
"#;

// The redirect URI is registered with Discord and must match byte for byte. Building
// it from the hostname rather than configuring it separately removes the way those
// two drift apart, which fails as an opaque OAuth error rather than a config one.
#[test]
fn the_redirect_uri_is_derived_from_the_hostname() {
    let config = RelayConfig::from_hcl(WITH_HTTP).expect("parses");

    assert_eq!(
        config.http.redirect_uri(),
        "https://registry.bedrockvoicechat.com/oauth/callback"
    );
}

// A labelled block, so the label names the provider. A second provider becomes an
// additional label rather than a breaking change to this one.
#[test]
fn the_acme_provider_is_named_by_its_block_label() {
    let config = RelayConfig::from_hcl(WITH_HTTP).expect("parses");

    let acme = config.http.cloudflare().expect("a cloudflare block");
    assert_eq!(acme.email, "ops@example.com");
    assert_eq!(acme.api_token, "cf-multi-zone-token");
}

// Production by default. A registry that quietly ordered from staging would serve a
// certificate no browser trusts, and the symptom is a TLS warning rather than
// anything naming the directory.
#[test]
fn the_acme_directory_defaults_to_production() {
    let config = RelayConfig::from_hcl(WITH_HTTP).expect("parses");

    assert_eq!(
        config.http.cloudflare().expect("a cloudflare block").directory,
        "https://acme-v02.api.letsencrypt.org/directory"
    );
}

// iroh and its transport log heavily below `info` and bury every line an operator
// needs. A bare level would silence the registry too, or drown it — the directives are
// what keep `debug` usable for chasing an enrollment.
#[test]
fn a_verbose_level_still_holds_the_transport_down() {
    let logger = bvc_relay_service::config::LoggerConfig {
        level: "debug".to_string(),
        path: "/tmp/logs".to_string(),
    };

    let directives = logger.directives();
    assert!(directives.starts_with("debug"));
    assert!(directives.contains("iroh=info"));
}

// An unrecognised level reads as `info` rather than as silence. A typo that turned
// logging off would be discovered only when something needed diagnosing.
#[test]
fn an_unknown_level_falls_back_to_info() {
    let logger = bvc_relay_service::config::LoggerConfig {
        level: "chatty".to_string(),
        path: "/tmp/logs".to_string(),
    };

    assert!(logger.directives().starts_with("info"));
}

// Console is unconditional and the file is configured. A config with no logger block
// still logs, because a registry that came up silent is one nobody can diagnose.
#[test]
fn a_config_without_a_logger_block_still_has_a_level_and_a_path() {
    let config = RelayConfig::from_hcl(WITH_HTTP).expect("parses");

    assert_eq!(config.logger.level, "info");
    assert!(!config.logger.path.is_empty());
}
