//! POST /api/admin/user — create a player.
//!
//! Contract:
//! - 401 when no client cert is presented
//! - 403 when the caller has a cert but lacks the `admin` permission
//! - 403 when the caller is admin but banished
//! - 201 on a fresh gamertag/game pair
//! - 409 on duplicate (gamertag, game)

mod harness;

use harness::http_client::MtlsClient;
use harness::server::{ADMIN_GAME, ADMIN_GAMERTAG};
use harness::{assert_status, TestServer};

use common::request::admin::CreateUserRequest;
use common::Game;

const ENDPOINT: &str = "/api/admin/user";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn returns_401_without_client_cert() {
    let env = TestServer::start().await.unwrap();
    let client = env.noauth_client().unwrap();

    let resp = client
        .post(format!("{}{}", env.base_url, ENDPOINT))
        .json(&CreateUserRequest {
            gamertag: "Bob".into(),
            game: Game::Minecraft,
        })
        .send()
        .await
        .unwrap();

    assert_status(resp.status().as_u16(), 401);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn returns_403_for_non_admin_caller() {
    let env = TestServer::start().await.unwrap();
    let (cert, key) = env.issue_player("Bob", &Game::Minecraft).await.unwrap();
    let client = MtlsClient::with_identity(&env.ca_pem, &cert, &key).unwrap();

    let resp = client
        .post(format!("{}{}", env.base_url, ENDPOINT))
        .json(&CreateUserRequest {
            gamertag: "Charlie".into(),
            game: Game::Minecraft,
        })
        .send()
        .await
        .unwrap();

    assert_status(resp.status().as_u16(), 403);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn returns_403_for_banished_admin() {
    let env = TestServer::start().await.unwrap();
    env.mark_banished(ADMIN_GAMERTAG, &ADMIN_GAME, true)
        .await
        .unwrap();

    let resp = env
        .admin_client()
        .unwrap()
        .post(format!("{}{}", env.base_url, ENDPOINT))
        .json(&CreateUserRequest {
            gamertag: "Charlie".into(),
            game: Game::Minecraft,
        })
        .send()
        .await
        .unwrap();

    assert_status(resp.status().as_u16(), 403);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn returns_201_on_fresh_gamertag() {
    let env = TestServer::start().await.unwrap();
    let resp = env
        .admin_client()
        .unwrap()
        .post(format!("{}{}", env.base_url, ENDPOINT))
        .json(&CreateUserRequest {
            gamertag: "Diana".into(),
            game: Game::Minecraft,
        })
        .send()
        .await
        .unwrap();

    assert_status(resp.status().as_u16(), 201);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["gamertag"].as_str().unwrap(), "Diana");
    assert_eq!(body["game"].as_str().unwrap(), "minecraft");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn returns_409_on_duplicate() {
    let env = TestServer::start().await.unwrap();
    let client = env.admin_client().unwrap();

    let req = CreateUserRequest {
        gamertag: "Eve".into(),
        game: Game::Minecraft,
    };

    let first = client
        .post(format!("{}{}", env.base_url, ENDPOINT))
        .json(&req)
        .send()
        .await
        .unwrap();
    assert_status(first.status().as_u16(), 201);

    let second = client
        .post(format!("{}{}", env.base_url, ENDPOINT))
        .json(&req)
        .send()
        .await
        .unwrap();
    assert_status(second.status().as_u16(), 409);
}
