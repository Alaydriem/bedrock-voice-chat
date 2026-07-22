use crate::harness::TestServer;
use common::structs::control::{ClientAction, ClientActionType};

const TOKEN: &str = "test-mc-token";

#[tokio::test]
async fn control_rejects_missing_token() {
    let env = TestServer::start().await.unwrap();
    let client = env.noauth_client().unwrap();
    let body = ClientAction {
        id: "Alice".into(),
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
