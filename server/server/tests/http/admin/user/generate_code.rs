//! POST /api/admin/user/code
//!
//! Contract:
//! - 401 without client cert
//! - 403 for non-admin
//! - 404 when target doesn't exist
//! - 200 with `{ code, expires_in_seconds }` for an existing player

use crate::harness::http_client::MtlsClient;
use crate::harness::{assert_status, TestServer};

use common::request::admin::GenerateCodeRequest;
use common::Game;

const ENDPOINT: &str = "/api/admin/user/code";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn returns_401_without_client_cert() {
    let env = TestServer::start().await.unwrap();
    let resp = env
        .noauth_client()
        .unwrap()
        .post(format!("{}{}", env.base_url, ENDPOINT))
        .json(&GenerateCodeRequest {
            gamertag: "Bob".into(),
            game: Game::Minecraft,
            duration: 60,
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
        .post(format!("{}{}", env.base_url, ENDPOINT))
        .json(&GenerateCodeRequest {
            gamertag: "Bob".into(),
            game: Game::Minecraft,
            duration: 60,
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
        .post(format!("{}{}", env.base_url, ENDPOINT))
        .json(&GenerateCodeRequest {
            gamertag: "Ghost".into(),
            game: Game::Minecraft,
            duration: 60,
        })
        .send()
        .await
        .unwrap();
    assert_status(resp.status().as_u16(), 404);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn admin_can_generate_code_for_existing_player() {
    let env = TestServer::start().await.unwrap();
    let _ = env.issue_player("Bob", &Game::Minecraft).await.unwrap();
    let resp = env
        .admin_client()
        .unwrap()
        .post(format!("{}{}", env.base_url, ENDPOINT))
        .json(&GenerateCodeRequest {
            gamertag: "Bob".into(),
            game: Game::Minecraft,
            duration: 600,
        })
        .send()
        .await
        .unwrap();
    assert_status(resp.status().as_u16(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    let code = body["code"].as_str().unwrap();
    assert!(!code.is_empty(), "code should be non-empty");
    assert_eq!(body["expires_in_seconds"].as_u64().unwrap(), 600);
}
