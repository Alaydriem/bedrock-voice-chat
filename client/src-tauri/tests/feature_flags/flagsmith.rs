use bvc_client_lib::FlagsmithProvider;

#[test]
fn identity_persists_but_build_number_trait_is_transient() {
    let body = FlagsmithProvider::build_identity_body("install-abc", 1001084);

    assert!(
        body.get("transient").is_none() || body["transient"] == serde_json::json!(false),
        "the install identity must persist; only the build_number trait is transient"
    );

    let traits = body["traits"].as_array().expect("traits array");
    let build = traits
        .iter()
        .find(|t| t["trait_key"] == serde_json::json!("build_number"))
        .expect("build_number trait");

    assert!(
        build["trait_value"].is_i64(),
        "build_number must serialize as an integer"
    );
    assert_eq!(build["trait_value"], serde_json::json!(1001084));
    assert_eq!(build["transient"], serde_json::json!(true));
}
