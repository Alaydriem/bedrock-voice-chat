use std::collections::HashMap;

use bvc_server_lib::config::ApplicationConfig;

// Minimal valid document plus whatever log block the test is about.
fn doc(log_block: &str) -> String {
    format!(
        r#"
server {{
    port = 8444
    minecraft {{
        access_token = "literal-token"
    }}
}}
{log_block}
"#
    )
}

fn parse(log_block: &str) -> Result<ApplicationConfig, anyhow::Error> {
    ApplicationConfig::from_hcl_str_with_env(&doc(log_block), &HashMap::new())
}

#[test]
fn a_config_that_still_names_out_is_rejected() {
    let err = parse("log {\n  level = \"info\"\n  out = \"/var/log/bvc\"\n}")
        .expect_err("the removed key must be a hard error, not a silently ignored one");

    let message = err.to_string();
    assert!(
        message.contains("out"),
        "the error must name the removed key, got: {message}"
    );
}

#[test]
fn the_log_path_defaults_to_a_logs_directory() {
    let config = parse("log {\n  level = \"info\"\n}").expect("a bare log block must parse");

    assert_eq!(config.log.path, "./logs");
}

#[test]
fn an_explicit_log_path_is_carried_through() {
    let config =
        parse("log {\n  level = \"debug\"\n  path = \"/var/log/bvc\"\n}").expect("must parse");

    assert_eq!(config.log.path, "/var/log/bvc");
    assert_eq!(config.log.level, "debug");
}
