use bvc_server_lib::config::ApplicationConfig;

// A document with no limits block. Absence must read as unlimited: every deployment that
// predates this feature keeps admitting everyone.
const NO_LIMITS_BLOCK: &str = r#"
server {
    port = 8444
    minecraft {
        access_token = "test-token"
    }
}
"#;

const LIMITS_SET: &str = r#"
server {
    port = 8444
    minecraft {
        access_token = "test-token"
    }
}
voice {
    limits {
        connections = 20
        reconnect_grace = 90
    }
}
"#;

// Only the limit, relying on the grace default. An operator sizing an instance has no
// reason to think about the reconnect window.
const ONLY_CONNECTIONS: &str = r#"
server {
    port = 8444
    minecraft {
        access_token = "test-token"
    }
}
voice {
    limits {
        connections = 5
    }
}
"#;

// `voice` is a sibling of `server`, not a child of it. Nested inside `server` the block
// deserializes into nothing — no error, no warning, and the limit silently never applies.
const LIMITS_NESTED_UNDER_SERVER: &str = r#"
server {
    port = 8444
    minecraft {
        access_token = "test-token"
    }
    voice {
        limits {
            connections = 3
        }
    }
}
"#;

#[test]
fn absent_limits_block_is_unlimited() {
    let cfg = ApplicationConfig::from_hcl_str_with_env(NO_LIMITS_BLOCK, &Default::default())
        .expect("document without a limits block must parse");
    assert_eq!(
        cfg.voice.limits.connections, 0,
        "a server that never configured a limit must keep admitting everyone"
    );
}

#[test]
fn both_limit_values_reach_the_config() {
    let cfg = ApplicationConfig::from_hcl_str_with_env(LIMITS_SET, &Default::default())
        .expect("document must parse");
    assert_eq!(cfg.voice.limits.connections, 20);
    assert_eq!(cfg.voice.limits.reconnect_grace, 90);
}

#[test]
fn connections_alone_keeps_the_default_grace() {
    let cfg = ApplicationConfig::from_hcl_str_with_env(ONLY_CONNECTIONS, &Default::default())
        .expect("document must parse");
    assert_eq!(cfg.voice.limits.connections, 5);
    assert_eq!(
        cfg.voice.limits.reconnect_grace, 60,
        "an operator who sets only the limit gets the default window"
    );
}

#[test]
fn a_limits_block_nested_under_server_does_not_apply() {
    let cfg =
        ApplicationConfig::from_hcl_str_with_env(LIMITS_NESTED_UNDER_SERVER, &Default::default())
            .expect("document parses");
    assert_eq!(
        cfg.voice.limits.connections, 0,
        "misplacing the block must be visible here rather than silently taking effect"
    );
}
