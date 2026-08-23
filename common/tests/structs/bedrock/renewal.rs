use common::bedrock_protocol::Error;
use common::structs::bedrock::BedrockRenewal;

#[test]
fn a_rejected_credential_asks_for_re_authentication() {
    let error = Error::ReauthRequired {
        detail: "token expired".to_string(),
    };
    assert!(matches!(
        BedrockRenewal::from(&error),
        BedrockRenewal::ReauthRequired
    ));
}

#[test]
fn every_other_auth_failure_is_transient() {
    let error = Error::Auth("token refresh failed (503): upstream".to_string());
    match BedrockRenewal::from(&error) {
        BedrockRenewal::Unavailable { message } => assert!(message.contains("503")),
        other => panic!("expected Unavailable, got {other:?}"),
    }
}

#[test]
fn a_transport_failure_is_transient() {
    let error = Error::RakNet("no route to host".to_string());
    assert!(matches!(
        BedrockRenewal::from(&error),
        BedrockRenewal::Unavailable { .. }
    ));
}
