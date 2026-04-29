//! PUT /api/admin/permission
//!
//! Contract:
//! - 401 / 403 admin gate
//! - 404 when target doesn't exist
//! - 400 for unknown permission name
//! - 204 on successful set
//! - 409 when admin tries to deny their own admin permission

mod harness;

use harness::server::{ADMIN_GAME, ADMIN_GAMERTAG};
use harness::{assert_status, TestServer};

use common::request::admin::SetPermissionRequest;
use common::structs::permission::PermissionEffect;
use common::Game;

const ENDPOINT: &str = "/api/admin/permission";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn returns_401_without_client_cert() {
    let env = TestServer::start().await.unwrap();
    let resp = env
        .noauth_client()
        .unwrap()
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
    assert_status(resp.status().as_u16(), 401);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn returns_404_when_target_missing() {
    let env = TestServer::start().await.unwrap();
    let resp = env
        .admin_client()
        .unwrap()
        .put(format!("{}{}", env.base_url, ENDPOINT))
        .json(&SetPermissionRequest {
            gamertag: "Ghost".into(),
            game: Game::Minecraft,
            permission: "audio_upload".into(),
            effect: PermissionEffect::Allow,
        })
        .send()
        .await
        .unwrap();
    assert_status(resp.status().as_u16(), 404);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn returns_400_for_unknown_permission() {
    let env = TestServer::start().await.unwrap();
    let _ = env.issue_player("Bob", &Game::Minecraft).await.unwrap();
    let resp = env
        .admin_client()
        .unwrap()
        .put(format!("{}{}", env.base_url, ENDPOINT))
        .json(&SetPermissionRequest {
            gamertag: "Bob".into(),
            game: Game::Minecraft,
            permission: "not_a_real_permission".into(),
            effect: PermissionEffect::Allow,
        })
        .send()
        .await
        .unwrap();
    assert_status(resp.status().as_u16(), 400);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn admin_can_grant_permission_to_other_player() {
    let env = TestServer::start().await.unwrap();
    let _ = env.issue_player("Bob", &Game::Minecraft).await.unwrap();
    let resp = env
        .admin_client()
        .unwrap()
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
    assert_status(resp.status().as_u16(), 204);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rejects_self_admin_deny() {
    let env = TestServer::start().await.unwrap();
    let resp = env
        .admin_client()
        .unwrap()
        .put(format!("{}{}", env.base_url, ENDPOINT))
        .json(&SetPermissionRequest {
            gamertag: ADMIN_GAMERTAG.into(),
            game: ADMIN_GAME.clone(),
            permission: "admin".into(),
            effect: PermissionEffect::Deny,
        })
        .send()
        .await
        .unwrap();
    assert_status(resp.status().as_u16(), 409);
}
