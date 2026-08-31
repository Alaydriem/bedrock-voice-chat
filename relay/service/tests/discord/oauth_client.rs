use bvc_relay_service::config::DiscordConfig;
use bvc_relay_service::discord::DiscordOAuthClient;
use serde_json::json;

use crate::harness::{MockApi, MockRoute};

fn config() -> DiscordConfig {
    DiscordConfig {
        guild_id: "guild".to_string(),
        bot_token: "bot".to_string(),
        client_id: "client-123".to_string(),
        client_secret: "the-secret".to_string(),
        qualifying_role_ids: vec!["role-a".to_string()],
    }
}

fn client(base: &str) -> DiscordOAuthClient {
    DiscordOAuthClient::new_with_base(
        &config(),
        "https://registry.example/oauth/callback".to_string(),
        base,
    )
}

// The exchange is two calls whose answers feed each other. Everything downstream —
// entitlement, the assigned name — keys on the id this returns, so reading it out of
// the wrong field would attribute one member's enrollment to another.
#[tokio::test]
async fn the_exchange_yields_the_discord_user_id() {
    let mock = MockApi::start(vec![
        MockRoute::new(
            "POST",
            "/oauth2/token",
            json!({ "access_token": "at-1", "token_type": "Bearer" }),
        ),
        MockRoute::new("GET", "/users/@me", json!({ "id": "member-9", "username": "someone" })),
    ])
    .await;

    let id = client(&mock.base)
        .identify("the-code")
        .await
        .expect("identifies");

    assert_eq!(id, "member-9");
}

// The client secret goes in the exchange body, and the redirect URI has to match what
// Discord has registered. A mismatch is refused by Discord with an error that names
// neither field.
#[tokio::test]
async fn the_exchange_sends_the_code_grant_with_the_registered_redirect() {
    let mock = MockApi::start(vec![
        MockRoute::new("POST", "/oauth2/token", json!({ "access_token": "at-1" })),
        MockRoute::new("GET", "/users/@me", json!({ "id": "member-9" })),
    ])
    .await;

    client(&mock.base).identify("the-code").await.expect("identifies");

    let exchange = mock
        .requests()
        .into_iter()
        .find(|r| r.path == "/oauth2/token")
        .expect("the token exchange happened");

    assert!(exchange.body.contains("grant_type=authorization_code"));
    assert!(exchange.body.contains("code=the-code"));
    assert!(exchange.body.contains("client_id=client-123"));
    assert!(exchange.body.contains("client_secret=the-secret"));
    assert!(exchange.body.contains("oauth%2Fcallback"));
}

// A refused exchange answers 200 with an error object rather than a failure status.
// Reading the missing field as absent rather than unwrapping it is what keeps a bad
// code from taking down the callback.
#[tokio::test]
async fn a_refused_exchange_is_an_error_not_a_panic() {
    let mock = MockApi::start(vec![MockRoute::new(
        "POST",
        "/oauth2/token",
        json!({ "error": "invalid_grant" }),
    )])
    .await;

    let error = client(&mock.base)
        .identify("a-stale-code")
        .await
        .expect_err("a refused exchange is an error");

    assert!(error.to_string().contains("access_token"));
}

// The user payload is the other half. A response without an id must not be read as an
// enrollment for an empty account.
#[tokio::test]
async fn a_user_payload_without_an_id_is_an_error() {
    let mock = MockApi::start(vec![
        MockRoute::new("POST", "/oauth2/token", json!({ "access_token": "at-1" })),
        MockRoute::new("GET", "/users/@me", json!({ "message": "401: Unauthorized" })),
    ])
    .await;

    assert!(client(&mock.base).identify("the-code").await.is_err());
}
