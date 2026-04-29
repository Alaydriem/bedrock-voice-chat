//! POST /api/auth/code/json
//!
//! Plain-JSON sibling of `/api/auth/code` (which is ncryptf-wrapped). Used by the CLI
//! `bvc login` flow: a player presents their one-time code and receives their issued
//! mTLS cert/key/CA bundle plus other identity material.
//!
//! Contract:
//! - 200 + LoginResponse on a fresh, unexpired code matching the gamertag
//! - 404 on unknown code
//! - 403 on gamertag mismatch / already-used code
//! - 410 (Gone) on expired code

mod harness;

use harness::{assert_status, TestServer};

use bvc_server_lib::services::AuthCodeService;
use common::request::CodeLoginRequest;
use common::Game;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

const ENDPOINT: &str = "/api/auth/code/json";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn returns_404_on_unknown_code() {
    let env = TestServer::start().await.unwrap();
    let resp = env
        .noauth_client()
        .unwrap()
        .post(format!("{}{}", env.base_url, ENDPOINT))
        .json(&CodeLoginRequest {
            gamertag: "Bob".into(),
            code: "DEFINITELY-NOT-A-REAL-CODE".into(),
        })
        .send()
        .await
        .unwrap();
    assert_status(resp.status().as_u16(), 404);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn returns_login_response_on_valid_code() {
    let env = TestServer::start().await.unwrap();
    let bob = env
        .issue_player("Bob", &Game::Minecraft)
        .await
        .unwrap();
    drop(bob);

    let bob_player = entity::player::Entity::find()
        .filter(entity::player::Column::Gamertag.eq("Bob"))
        .one(&env.db)
        .await
        .unwrap()
        .expect("Bob should exist");

    let code = AuthCodeService::generate_code(&env.db, bob_player.id, 600)
        .await
        .unwrap();

    let resp = env
        .noauth_client()
        .unwrap()
        .post(format!("{}{}", env.base_url, ENDPOINT))
        .json(&CodeLoginRequest {
            gamertag: "Bob".into(),
            code,
        })
        .send()
        .await
        .unwrap();
    assert_status(resp.status().as_u16(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["gamertag"].as_str().unwrap(), "Bob");
    assert!(!body["certificate"].as_str().unwrap().is_empty());
    assert!(!body["certificate_key"].as_str().unwrap().is_empty());
    assert!(!body["certificate_ca"].as_str().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn returns_403_on_gamertag_mismatch() {
    let env = TestServer::start().await.unwrap();
    let _ = env.issue_player("Bob", &Game::Minecraft).await.unwrap();

    let bob_player = entity::player::Entity::find()
        .filter(entity::player::Column::Gamertag.eq("Bob"))
        .one(&env.db)
        .await
        .unwrap()
        .expect("Bob should exist");

    let code = AuthCodeService::generate_code(&env.db, bob_player.id, 600)
        .await
        .unwrap();

    // Use Bob's code but claim to be Alice — mismatch.
    let resp = env
        .noauth_client()
        .unwrap()
        .post(format!("{}{}", env.base_url, ENDPOINT))
        .json(&CodeLoginRequest {
            gamertag: "Alice".into(),
            code,
        })
        .send()
        .await
        .unwrap();
    assert_status(resp.status().as_u16(), 403);
}
