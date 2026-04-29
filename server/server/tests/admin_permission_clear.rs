//! DELETE /api/admin/permission
//!
//! Contract:
//! - 401 admin gate
//! - 404 when target doesn't exist
//! - 404 when no override is set for that permission
//! - 204 on successful clear
//! - 409 when admin tries to clear their own admin permission

mod harness;

use harness::server::{ADMIN_GAME, ADMIN_GAMERTAG};
use harness::{assert_status, TestServer};

use common::request::admin::{ClearPermissionRequest, SetPermissionRequest};
use common::structs::permission::PermissionEffect;
use common::Game;

const ENDPOINT: &str = "/api/admin/permission";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn returns_401_without_client_cert() {
    let env = TestServer::start().await.unwrap();
    let resp = env
        .noauth_client()
        .unwrap()
        .delete(format!("{}{}", env.base_url, ENDPOINT))
        .json(&ClearPermissionRequest {
            gamertag: "Bob".into(),
            game: Game::Minecraft,
            permission: "audio_upload".into(),
        })
        .send()
        .await
        .unwrap();
    assert_status(resp.status().as_u16(), 401);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn returns_404_when_target_missing() {
    let env = TestServer::start().await.unwrap();
    let resp = env
        .admin_client()
        .unwrap()
        .delete(format!("{}{}", env.base_url, ENDPOINT))
        .json(&ClearPermissionRequest {
            gamertag: "Ghost".into(),
            game: Game::Minecraft,
            permission: "audio_upload".into(),
        })
        .send()
        .await
        .unwrap();
    assert_status(resp.status().as_u16(), 404);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn returns_404_when_no_override_exists() {
    let env = TestServer::start().await.unwrap();
    let _ = env.issue_player("Bob", &Game::Minecraft).await.unwrap();
    let resp = env
        .admin_client()
        .unwrap()
        .delete(format!("{}{}", env.base_url, ENDPOINT))
        .json(&ClearPermissionRequest {
            gamertag: "Bob".into(),
            game: Game::Minecraft,
            permission: "audio_upload".into(),
        })
        .send()
        .await
        .unwrap();
    assert_status(resp.status().as_u16(), 404);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn admin_can_clear_existing_override() {
    let env = TestServer::start().await.unwrap();
    let _ = env.issue_player("Bob", &Game::Minecraft).await.unwrap();
    let client = env.admin_client().unwrap();

    // Set first
    let set = client
        .put(format!("{}{}", env.base_url, ENDPOINT))
        .json(&SetPermissionRequest {
            gamertag: "Bob".into(),
            game: Game::Minecraft,
            permission: "audio_upload".into(),
            effect: PermissionEffect::Allow,
        })
        .send()
        .await
        .unwrap();
    assert_status(set.status().as_u16(), 204);

    // Then clear
    let clear = client
        .delete(format!("{}{}", env.base_url, ENDPOINT))
        .json(&ClearPermissionRequest {
            gamertag: "Bob".into(),
            game: Game::Minecraft,
            permission: "audio_upload".into(),
        })
        .send()
        .await
        .unwrap();
    assert_status(clear.status().as_u16(), 204);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rejects_self_admin_clear() {
    let env = TestServer::start().await.unwrap();
    let resp = env
        .admin_client()
        .unwrap()
        .delete(format!("{}{}", env.base_url, ENDPOINT))
        .json(&ClearPermissionRequest {
            gamertag: ADMIN_GAMERTAG.into(),
            game: ADMIN_GAME.clone(),
            permission: "admin".into(),
        })
        .send()
        .await
        .unwrap();
    assert_status(resp.status().as_u16(), 409);
}
