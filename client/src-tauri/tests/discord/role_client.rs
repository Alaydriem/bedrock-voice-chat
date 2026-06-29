use bvc_client_lib::DiscordRoleClient;

#[test]
fn roles_extracted_from_member_object() {
    let body = serde_json::json!({ "roles": ["111", "222"], "user": { "id": "9" } });
    assert_eq!(
        DiscordRoleClient::roles_from_member(&body),
        vec!["111".to_string(), "222".to_string()]
    );
}

#[test]
fn not_a_member_or_missing_roles_yields_empty() {
    // No roles field (e.g. error payload) must fail-closed to an empty list so
    // the feature stays locked rather than unlocking on malformed input.
    let body = serde_json::json!({ "message": "Unknown Guild", "code": 10004 });
    assert!(DiscordRoleClient::roles_from_member(&body).is_empty());
}
