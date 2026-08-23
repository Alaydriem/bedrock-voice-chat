mod database;
mod from_json;
mod schema;
mod server;
mod voice;

use std::collections::HashMap;

use bvc_server_lib::config::ApplicationConfig;

fn env_map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

// Minimal valid document: `server.minecraft` is the only required block today.
fn hcl_doc(token_expr: &str) -> String {
    format!(
        r#"
server {{
    port = 8444
    minecraft {{
        access_token = "{token_expr}"
    }}
}}
"#
    )
}

#[test]
fn from_hcl_str_with_env_interpolates_variables() {
    let doc = hcl_doc("${env.TEST_BVC_TOKEN}");
    let cfg = ApplicationConfig::from_hcl_str_with_env(
        &doc,
        &env_map(&[("TEST_BVC_TOKEN", "sekret-value")]),
    )
    .expect("interpolated document must parse");
    assert_eq!(cfg.server.minecraft.access_token, "sekret-value");
    assert_eq!(cfg.server.port, 8444);
}

#[test]
fn from_hcl_str_with_env_errors_on_unset_variable() {
    let doc = hcl_doc("${env.TEST_BVC_MISSING}");
    let err = ApplicationConfig::from_hcl_str_with_env(&doc, &env_map(&[]))
        .expect_err("unset variable must be a hard error");
    let msg = format!("{err}").to_lowercase();
    assert!(
        msg.contains("test_bvc_missing") || msg.contains("no such key") || msg.contains("undefined"),
        "error should identify the problem, got: {msg}"
    );
}

#[test]
fn from_hcl_str_with_env_parses_plain_documents_unchanged() {
    let doc = hcl_doc("literal-token");
    let cfg = ApplicationConfig::from_hcl_str_with_env(&doc, &env_map(&[]))
        .expect("plain document must parse");
    assert_eq!(cfg.server.minecraft.access_token, "literal-token");
}

#[test]
fn from_hcl_str_with_env_rejects_malformed_hcl() {
    let err = ApplicationConfig::from_hcl_str_with_env("server {", &env_map(&[]))
        .expect_err("malformed HCL must error");
    assert!(!format!("{err}").is_empty());
}
