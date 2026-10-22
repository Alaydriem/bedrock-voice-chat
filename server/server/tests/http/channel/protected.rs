//! `POST /api/channel/protected` and `PUT /api/channel/<id>/protected`
//!
//! Contract:
//! - a protected channel is created without its creator, who joins through the protected route
//! - the legacy Join refuses a protected channel, so a client that cannot run admission never
//!   becomes a member and never receives a sealed frame
//! - the protected Join refuses an unprotected channel

use common::Game;
use common::request::channel::ProtectedChannelCreateRequest;
use common::structs::channel::{Channel, ChannelEvent, ChannelEvents};

use crate::harness::http_client::MtlsClient;
use crate::harness::{HttpAssert, TestServer};

async fn create_protected(env: &TestServer, cert: &str, key: &str) -> String {
    let response = MtlsClient::with_identity(&env.ca_pem, cert, key)
        .unwrap()
        .post(format!("{}/api/channel/protected", env.base_url))
        .json(&ProtectedChannelCreateRequest {
            name: "vault".to_string(),
        })
        .send()
        .await
        .unwrap();
    HttpAssert::status(response.status().as_u16(), 200);
    response.text().await.unwrap().trim_matches('"').to_string()
}

async fn create_open(env: &TestServer, cert: &str, key: &str) -> String {
    let response = MtlsClient::with_identity(&env.ca_pem, cert, key)
        .unwrap()
        .post(format!("{}/api/channel", env.base_url))
        .json(&"open".to_string())
        .send()
        .await
        .unwrap();
    HttpAssert::status(response.status().as_u16(), 200);
    response.text().await.unwrap().trim_matches('"').to_string()
}

async fn fetch(env: &TestServer, cert: &str, key: &str, id: &str) -> Channel {
    let channels: Vec<Channel> = MtlsClient::with_identity(&env.ca_pem, cert, key)
        .unwrap()
        .get(format!("{}/api/channel?id={}", env.base_url, id))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    channels.into_iter().next().expect("channel listed")
}

async fn protected_join(env: &TestServer, cert: &str, key: &str, id: &str) -> u16 {
    MtlsClient::with_identity(&env.ca_pem, cert, key)
        .unwrap()
        .put(format!("{}/api/channel/{}/protected", env.base_url, id))
        .send()
        .await
        .unwrap()
        .status()
        .as_u16()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_protected_returns_a_protected_channel() {
    let env = TestServer::start().await.unwrap();
    let (cert, key) = env.issue_player("Creator", &Game::Minecraft).await.unwrap();

    let id = create_protected(&env, &cert, &key).await;
    let channel = fetch(&env, &cert, &key, &id).await;

    assert!(channel.protected);
    assert!(channel.players.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn legacy_join_refuses_a_protected_channel() {
    let env = TestServer::start().await.unwrap();
    let (creator, creator_key) = env.issue_player("Creator", &Game::Minecraft).await.unwrap();
    let (old, old_key) = env.issue_player("OldClient", &Game::Minecraft).await.unwrap();
    let id = create_protected(&env, &creator, &creator_key).await;

    let status = MtlsClient::with_identity(&env.ca_pem, &old, &old_key)
        .unwrap()
        .put(format!("{}/api/channel/{}", env.base_url, id))
        .json(&ChannelEvent::new(ChannelEvents::Join))
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();

    HttpAssert::status(status, 403);
    assert!(fetch(&env, &creator, &creator_key, &id).await.players.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn protected_join_adds_the_member() {
    let env = TestServer::start().await.unwrap();
    let (creator, creator_key) = env.issue_player("Creator", &Game::Minecraft).await.unwrap();
    let id = create_protected(&env, &creator, &creator_key).await;

    let status = protected_join(&env, &creator, &creator_key, &id).await;

    HttpAssert::status(status, 200);
    let players = fetch(&env, &creator, &creator_key, &id).await.players;
    assert_eq!(players.len(), 1);
    assert_eq!(players[0].to_string(), "minecraft:Creator");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn protected_join_refuses_an_unprotected_channel() {
    let env = TestServer::start().await.unwrap();
    let (creator, creator_key) = env.issue_player("Creator", &Game::Minecraft).await.unwrap();
    let id = create_open(&env, &creator, &creator_key).await;

    let status = protected_join(&env, &creator, &creator_key, &id).await;

    HttpAssert::status(status, 409);
}

async fn legacy_join(env: &TestServer, cert: &str, key: &str, id: &str) {
    let status = MtlsClient::with_identity(&env.ca_pem, cert, key)
        .unwrap()
        .put(format!("{}/api/channel/{}", env.base_url, id))
        .json(&ChannelEvent::new(ChannelEvents::Join))
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    HttpAssert::status(status, 200);
}

async fn protect(env: &TestServer, cert: &str, key: &str, id: &str) -> (u16, String) {
    let response = MtlsClient::with_identity(&env.ca_pem, cert, key)
        .unwrap()
        .put(format!("{}/api/channel/{}/protect", env.base_url, id))
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    (status, response.text().await.unwrap_or_default())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn protect_marks_an_owner_only_channel_protected() {
    let env = TestServer::start().await.unwrap();
    let (creator, creator_key) = env.issue_player("Creator", &Game::Minecraft).await.unwrap();
    let id = create_open(&env, &creator, &creator_key).await;
    legacy_join(&env, &creator, &creator_key, &id).await;

    let (status, _) = protect(&env, &creator, &creator_key, &id).await;

    HttpAssert::status(status, 200);
    assert!(fetch(&env, &creator, &creator_key, &id).await.protected);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn protect_refuses_a_second_member() {
    let env = TestServer::start().await.unwrap();
    let (creator, creator_key) = env.issue_player("Creator", &Game::Minecraft).await.unwrap();
    let (other, other_key) = env.issue_player("Other", &Game::Minecraft).await.unwrap();
    let id = create_open(&env, &creator, &creator_key).await;
    legacy_join(&env, &creator, &creator_key, &id).await;
    legacy_join(&env, &other, &other_key, &id).await;

    let (status, body) = protect(&env, &creator, &creator_key, &id).await;

    HttpAssert::status(status, 409);
    assert!(body.contains("not_alone"), "body: {body}");
    let channel = fetch(&env, &creator, &creator_key, &id).await;
    assert!(!channel.protected);
    assert_eq!(channel.players.len(), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn protect_refuses_a_non_owner() {
    let env = TestServer::start().await.unwrap();
    let (creator, creator_key) = env.issue_player("Creator", &Game::Minecraft).await.unwrap();
    let (other, other_key) = env.issue_player("Other", &Game::Minecraft).await.unwrap();
    let id = create_open(&env, &creator, &creator_key).await;

    let (status, _) = protect(&env, &other, &other_key, &id).await;

    HttpAssert::status(status, 403);
    assert!(!fetch(&env, &creator, &creator_key, &id).await.protected);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn protect_refuses_an_already_protected_channel() {
    let env = TestServer::start().await.unwrap();
    let (creator, creator_key) = env.issue_player("Creator", &Game::Minecraft).await.unwrap();
    let id = create_protected(&env, &creator, &creator_key).await;

    let (status, body) = protect(&env, &creator, &creator_key, &id).await;

    HttpAssert::status(status, 409);
    assert!(body.contains("already_protected"), "body: {body}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn protect_of_a_missing_channel_is_not_found() {
    let env = TestServer::start().await.unwrap();
    let (creator, creator_key) = env.issue_player("Creator", &Game::Minecraft).await.unwrap();

    let (status, _) = protect(&env, &creator, &creator_key, "ZZZZZZ").await;

    HttpAssert::status(status, 404);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn protected_join_of_a_missing_channel_is_not_found() {
    let env = TestServer::start().await.unwrap();
    let (creator, creator_key) = env.issue_player("Creator", &Game::Minecraft).await.unwrap();

    let status = protected_join(&env, &creator, &creator_key, "ZZZZZZ").await;

    HttpAssert::status(status, 404);
}
