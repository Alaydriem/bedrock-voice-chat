//! PATCH /api/admin/user/banish
//!
//! Contract:
//! - 401 / 403 / banished — same admin-gate as every admin route
//! - 200 toggling banished on another player
//! - 404 when the target gamertag/game pair doesn't exist
//! - 409 when admin tries to banish themselves

mod harness;

use harness::http_client::MtlsClient;
use harness::server::{ADMIN_GAME, ADMIN_GAMERTAG};
use harness::{assert_status, TestServer};

use common::request::admin::BanishUserRequest;
use common::Game;

const ENDPOINT: &str = "/api/admin/user/banish";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn returns_401_without_client_cert() {
    let env = TestServer::start().await.unwrap();
    let resp = env
        .noauth_client()
        .unwrap()
        .patch(format!("{}{}", env.base_url, ENDPOINT))
        .json(&BanishUserRequest {
            gamertag: "Bob".into(),
            game: Game::Minecraft,
            banish: true,
        })
        .send()
        .await
        .unwrap();
    assert_status(resp.status().as_u16(), 401);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn returns_403_for_non_admin() {
    let env = TestServer::start().await.unwrap();
    let (cert, key) = env.issue_player("Bob", &Game::Minecraft).await.unwrap();
    let resp = MtlsClient::with_identity(&env.ca_pem, &cert, &key)
        .unwrap()
        .patch(format!("{}{}", env.base_url, ENDPOINT))
        .json(&BanishUserRequest {
            gamertag: "Bob".into(),
            game: Game::Minecraft,
            banish: true,
        })
        .send()
        .await
        .unwrap();
    assert_status(resp.status().as_u16(), 403);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn returns_404_when_target_missing() {
    let env = TestServer::start().await.unwrap();
    let resp = env
        .admin_client()
        .unwrap()
        .patch(format!("{}{}", env.base_url, ENDPOINT))
        .json(&BanishUserRequest {
            gamertag: "Ghost".into(),
            game: Game::Minecraft,
            banish: true,
        })
        .send()
        .await
        .unwrap();
    assert_status(resp.status().as_u16(), 404);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn admin_can_banish_other_player() {
    let env = TestServer::start().await.unwrap();
    let _ = env.issue_player("Bob", &Game::Minecraft).await.unwrap();
    let resp = env
        .admin_client()
        .unwrap()
        .patch(format!("{}{}", env.base_url, ENDPOINT))
        .json(&BanishUserRequest {
            gamertag: "Bob".into(),
            game: Game::Minecraft,
            banish: true,
        })
        .send()
        .await
        .unwrap();
    assert_status(resp.status().as_u16(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["banished"].as_bool().unwrap(), true);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rejects_self_banish() {
    let env = TestServer::start().await.unwrap();
    let resp = env
        .admin_client()
        .unwrap()
        .patch(format!("{}{}", env.base_url, ENDPOINT))
        .json(&BanishUserRequest {
            gamertag: ADMIN_GAMERTAG.into(),
            game: ADMIN_GAME.clone(),
            banish: true,
        })
        .send()
        .await
        .unwrap();
    assert_status(resp.status().as_u16(), 409);
}
