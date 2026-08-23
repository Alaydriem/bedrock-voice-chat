use bvc_server_lib::config::{ApplicationConfig, Features};

// The path an operator actually takes. The serde tests below prove the derive honours an
// explicit false; they do not prove an HCL block reaches the struct at all.
#[test]
fn an_hcl_features_block_reaches_the_struct() {
    let doc = r#"
server {
    features {
        chat = false
    }
}
"#;
    let cfg = ApplicationConfig::from_hcl_str(doc).expect("parse features block");
    assert!(!cfg.server.features.chat);
}

#[test]
fn telemetry_defaults_to_true_when_absent() {
    let features: Features = serde_json::from_str("{}").expect("deserialize empty features");
    assert!(features.telemetry);
}

#[test]
fn chat_defaults_to_true_when_absent() {
    let features: Features = serde_json::from_str("{}").expect("deserialize empty features");
    assert!(features.chat);
}

// The assertion that matters for a flag whose default is true: an operator writing the switch
// off must be distinguishable from an operator who never wrote it. A default that also fired
// on an explicit `false` would leave the switch inoperable, and asserting the default alone
// would not notice.
#[test]
fn chat_honours_an_explicit_false() {
    let features: Features =
        serde_json::from_str(r#"{"chat":false}"#).expect("deserialize features");
    assert!(!features.chat);
}
