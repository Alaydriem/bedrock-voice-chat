use std::sync::Arc;

use bvc_client_lib::bedrock::BedrockState;
use common::bedrock_protocol::{AuthManager, RealmsApi, RealmsEnvironment};

fn sign_in(state: &mut BedrockState) {
    state.apply_auth(
        Arc::new(AuthManager::offline()),
        RealmsApi::new("xsts-token", "user-hash", RealmsEnvironment::Retail),
        "xbl-token".to_string(),
        "user-hash".to_string(),
        "access-token".to_string(),
        Some("refresh-token".to_string()),
        "xuid-1".to_string(),
    );
}

#[test]
fn clear_auth_leaves_no_identity_for_the_next_session_to_inherit() {
    let mut state = BedrockState::new();
    sign_in(&mut state);
    assert!(state.is_authenticated());

    state.clear_auth();

    assert!(!state.is_authenticated());
    assert!(state.realms_api.is_none());
    assert!(state.xuid.is_none());
    assert!(state.refresh_token.is_none());
    assert!(state.access_token.is_none());
    assert!(state.xbl_token.is_none());
    assert!(state.user_hash.is_none());
}

#[test]
fn a_successful_sign_in_retires_a_pending_reauth() {
    let mut state = BedrockState::new();
    state.reauth_required = true;

    sign_in(&mut state);

    assert!(!state.reauth_required);
}

#[test]
fn clearing_retires_a_pending_reauth() {
    let mut state = BedrockState::new();
    state.reauth_required = true;

    state.clear_auth();

    assert!(!state.reauth_required);
}
