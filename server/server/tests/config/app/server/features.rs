use bvc_server_lib::config::Features;

#[test]
fn telemetry_defaults_to_true_when_absent() {
    let features: Features = serde_json::from_str("{}").expect("deserialize empty features");
    assert!(features.telemetry);
}
