//! `PUT /api/channel/<id>`
//!
//! Contract:
//! - Join stays open by design: the channel id is a share code, and knowing it is the credential
//! - Delete and Rename require the creator, matching `DELETE` and `PATCH` on the same id
//! - Delete removes the channel, so this route and `DELETE /<id>` cannot disagree about state
//! - an event for a channel that does not exist is 404 and fans nothing

use common::Game;
use common::structs::channel::{ChannelEvent, ChannelEvents};

use crate::harness::http_client::MtlsClient;
use crate::harness::{HttpAssert, TestServer};

async fn create_channel(env: &TestServer, cert: &str, key: &str, name: &str) -> String {
    let response = MtlsClient::with_identity(&env.ca_pem, cert, key)
        .unwrap()
        .post(format!("{}/api/channel", env.base_url))
        .json(&name.to_string())
        .send()
        .await
        .unwrap();
    HttpAssert::status(response.status().as_u16(), 200);
    response.text().await.unwrap().trim_matches('"').to_string()
}

async fn channel_exists(env: &TestServer, cert: &str, key: &str, id: &str) -> bool {
    let response = MtlsClient::with_identity(&env.ca_pem, cert, key)
        .unwrap()
        .get(format!("{}/api/channel?id={}", env.base_url, id))
        .send()
        .await
        .unwrap();
    response.status().as_u16() == 200
}

async fn send_event(
    env: &TestServer,
    cert: &str,
    key: &str,
    id: &str,
    event: ChannelEvents,
) -> u16 {
    MtlsClient::with_identity(&env.ca_pem, cert, key)
        .unwrap()
        .put(format!("{}/api/channel/{}", env.base_url, id))
        .json(&ChannelEvent::new(event))
        .send()
        .await
        .unwrap()
        .status()
        .as_u16()
}

// `DELETE /<id>` checks the creator; this route reached the same effect without one, so the
// guarded route was bypassable by anyone who knew the id.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_non_creator_cannot_delete_through_the_event_route() {
    let env = TestServer::start().await.unwrap();
    let (creator, creator_key) = env.issue_player("Creator", &Game::Minecraft).await.unwrap();
    let (outsider, outsider_key) = env.issue_player("Outsider", &Game::Minecraft).await.unwrap();
    let id = create_channel(&env, &creator, &creator_key, "private").await;

    let status = send_event(&env, &outsider, &outsider_key, &id, ChannelEvents::Delete).await;

    HttpAssert::status(status, 401);
    assert!(channel_exists(&env, &creator, &creator_key, &id).await);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_creator_can_delete_and_the_channel_is_removed() {
    let env = TestServer::start().await.unwrap();
    let (creator, creator_key) = env.issue_player("Creator", &Game::Minecraft).await.unwrap();
    let id = create_channel(&env, &creator, &creator_key, "doomed").await;

    let status = send_event(&env, &creator, &creator_key, &id, ChannelEvents::Delete).await;

    HttpAssert::status(status, 200);
    assert!(!channel_exists(&env, &creator, &creator_key, &id).await);
}

// The old code fanned a Delete for an id that never existed, which every connected client
// then processed.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_event_for_a_missing_channel_is_not_found() {
    let env = TestServer::start().await.unwrap();
    let (cert, key) = env.issue_player("Anyone", &Game::Minecraft).await.unwrap();

    let status = send_event(&env, &cert, &key, "does-not-exist", ChannelEvents::Delete).await;

    HttpAssert::status(status, 404);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_non_creator_cannot_rename_through_the_event_route() {
    let env = TestServer::start().await.unwrap();
    let (creator, creator_key) = env.issue_player("Creator", &Game::Minecraft).await.unwrap();
    let (outsider, outsider_key) = env.issue_player("Outsider", &Game::Minecraft).await.unwrap();
    let id = create_channel(&env, &creator, &creator_key, "original").await;

    let status = send_event(&env, &outsider, &outsider_key, &id, ChannelEvents::Rename).await;

    HttpAssert::status(status, 401);
}

// Join stays open by design. This test exists to stop the fix over-reaching: the id is a
// share code, and being handed one is the whole mechanism for joining a friend's group.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn any_player_may_still_join_a_channel_by_id() {
    let env = TestServer::start().await.unwrap();
    let (creator, creator_key) = env.issue_player("Creator", &Game::Minecraft).await.unwrap();
    let (joiner, joiner_key) = env.issue_player("Joiner", &Game::Minecraft).await.unwrap();
    let id = create_channel(&env, &creator, &creator_key, "open").await;

    let status = send_event(&env, &joiner, &joiner_key, &id, ChannelEvents::Join).await;

    HttpAssert::status(status, 200);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn any_player_may_leave_a_channel_they_joined() {
    let env = TestServer::start().await.unwrap();
    let (creator, creator_key) = env.issue_player("Creator", &Game::Minecraft).await.unwrap();
    let (joiner, joiner_key) = env.issue_player("Joiner", &Game::Minecraft).await.unwrap();
    let id = create_channel(&env, &creator, &creator_key, "open").await;
    send_event(&env, &joiner, &joiner_key, &id, ChannelEvents::Join).await;

    let status = send_event(&env, &joiner, &joiner_key, &id, ChannelEvents::Leave).await;

    HttpAssert::status(status, 200);
}
