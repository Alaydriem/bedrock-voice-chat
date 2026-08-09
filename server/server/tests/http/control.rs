use crate::harness::TestServer;
use common::structs::control::{ClientAction, ClientActionType};

const TOKEN: &str = "test-mc-token";

#[tokio::test]
async fn control_rejects_missing_token() {
    let env = TestServer::start().await.unwrap();
    let client = env.noauth_client().unwrap();
    let body = ClientAction {
        id: "Alice".into(),
        game: None,
        action: ClientActionType::CreateGroup,
    };
    let resp = client
        .post(format!("{}/api/control", env.base_url))
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_ne!(
        resp.status(),
        200,
        "a request without the mod token must be rejected"
    );
}

#[tokio::test]
async fn control_create_group_with_token_returns_ok() {
    let env = TestServer::start().await.unwrap();
    let client = env.noauth_client().unwrap();
    let body = ClientAction {
        id: "Alice".into(),
        game: None,
        action: ClientActionType::CreateGroup,
    };
    let resp = client
        .post(format!("{}/api/control", env.base_url))
        .header("X-MC-Access-Token", TOKEN)
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn get_state_for_unknown_player_is_ok_and_guarded() {
    let env = TestServer::start().await.unwrap();
    let client = env.noauth_client().unwrap();

    // Guarded:
    let no_token = client
        .get(format!("{}/api/state?id=ghost", env.base_url))
        .send()
        .await
        .unwrap();
    assert_ne!(no_token.status(), 200);

    // With token, an unknown player yields a 200 (empty state).
    let resp = client
        .get(format!("{}/api/state?id=ghost", env.base_url))
        .header("X-MC-Access-Token", TOKEN)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn control_join_unknown_group_is_not_found() {
    let env = TestServer::start().await.unwrap();
    let client = env.noauth_client().unwrap();
    let body = ClientAction {
        id: "Alice".into(),
        game: None,
        action: ClientActionType::JoinGroup {
            channel: "does-not-exist".into(),
        },
    };
    let resp = client
        .post(format!("{}/api/control", env.base_url))
        .header("X-MC-Access-Token", TOKEN)
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        404,
        "joining an unknown share code is a client error, not a server fault"
    );
}

#[tokio::test]
async fn control_refuses_to_arm_recording_when_the_server_disallows_it() {
    let env = TestServer::start_with_recording(false).await.unwrap();
    let client = env.noauth_client().unwrap();
    let body = ClientAction {
        id: "Alice".into(),
        game: None,
        action: ClientActionType::SetRecording(true),
    };
    let resp = client
        .post(format!("{}/api/control", env.base_url))
        .header("X-MC-Access-Token", TOKEN)
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        403,
        "arming a recording must be refused where the operator turned recording off"
    );
}

// Delivery is out of reach here: this harness boots no QUIC plane, so a self action
// that gets past the policy gate then answers 503 for want of a live connection to
// deliver to. What these two assert is the gate itself — refused, or not refused.
#[tokio::test]
async fn control_always_allows_stopping_a_recording() {
    let env = TestServer::start_with_recording(false).await.unwrap();
    let client = env.noauth_client().unwrap();
    let body = ClientAction {
        id: "Alice".into(),
        game: None,
        action: ClientActionType::SetRecording(false),
    };
    let resp = client
        .post(format!("{}/api/control", env.base_url))
        .header("X-MC-Access-Token", TOKEN)
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_ne!(
        resp.status(),
        403,
        "stopping must never be barred: an operator flipping the switch mid-session \
         must not strand someone in a recording they cannot end"
    );
}

#[tokio::test]
async fn control_arms_recording_where_the_server_allows_it() {
    let env = TestServer::start_with_recording(true).await.unwrap();
    let client = env.noauth_client().unwrap();
    let body = ClientAction {
        id: "Alice".into(),
        game: None,
        action: ClientActionType::SetRecording(true),
    };
    let resp = client
        .post(format!("{}/api/control", env.base_url))
        .header("X-MC-Access-Token", TOKEN)
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_ne!(
        resp.status(),
        403,
        "a server that permits recording must not refuse the action"
    );
}
