//! GET /api/admin/permission/<game>/<gamertag>
//!
//! Contract:
//! - 401 admin gate
//! - 404 when target doesn't exist
//! - 200 with empty entries when no overrides
//! - 200 with entries reflecting set overrides

use crate::harness::{HttpAssert, TestServer};

use common::Game;
use common::request::admin::SetPermissionRequest;
use common::structs::permission::PermissionEffect;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn returns_401_without_client_cert() {
    let env = TestServer::start().await.unwrap();
    let resp = env
        .noauth_client()
        .unwrap()
        .get(format!(
            "{}/api/admin/permission/minecraft/Bob",
            env.base_url
        ))
        .send()
        .await
        .unwrap();
    HttpAssert::status(resp.status().as_u16(), 401);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn returns_404_when_target_missing() {
    let env = TestServer::start().await.unwrap();
    let resp = env
        .admin_client()
        .unwrap()
        .get(format!(
            "{}/api/admin/permission/minecraft/Ghost",
            env.base_url
        ))
        .send()
        .await
        .unwrap();
    HttpAssert::status(resp.status().as_u16(), 404);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn returns_empty_entries_when_no_overrides() {
    let env = TestServer::start().await.unwrap();
    let _ = env.issue_player("Bob", &Game::Minecraft).await.unwrap();
    let resp = env
        .admin_client()
        .unwrap()
        .get(format!(
            "{}/api/admin/permission/minecraft/Bob",
            env.base_url
        ))
        .send()
        .await
        .unwrap();
    HttpAssert::status(resp.status().as_u16(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["gamertag"].as_str().unwrap(), "Bob");
    assert_eq!(body["entries"].as_array().unwrap().len(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn returns_set_overrides() {
    let env = TestServer::start().await.unwrap();
    let _ = env.issue_player("Bob", &Game::Minecraft).await.unwrap();
    let client = env.admin_client().unwrap();

    let set = client
        .put(format!("{}/api/admin/permission", env.base_url))
        .json(&SetPermissionRequest {
            gamertag: "Bob".into(),
            game: Game::Minecraft,
            permission: "audio_upload".into(),
            effect: PermissionEffect::Allow,
        })
        .send()
        .await
        .unwrap();
    HttpAssert::status(set.status().as_u16(), 204);

    let resp = client
        .get(format!(
            "{}/api/admin/permission/minecraft/Bob",
            env.base_url
        ))
        .send()
        .await
        .unwrap();
    HttpAssert::status(resp.status().as_u16(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    let entries = body["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["permission"].as_str().unwrap(), "audio_upload");
    assert_eq!(entries[0]["effect"].as_str().unwrap(), "allow");
}
