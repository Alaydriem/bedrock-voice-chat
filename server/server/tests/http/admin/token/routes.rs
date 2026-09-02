//! /api/admin/token
//!
//! Contract:
//! - 401 admin gate on every method
//! - POST returns a token exactly once
//! - DELETE retires it
//! - rotate swaps one credential for another
//! - the legacy route reports the configured scalar

use crate::harness::{HttpAssert, TestServer};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn listing_requires_an_admin_certificate() {
    let env = TestServer::start().await.unwrap();
    let resp = env
        .noauth_client()
        .unwrap()
        .get(format!("{}/api/admin/token", env.base_url))
        .send()
        .await
        .unwrap();
    HttpAssert::status(resp.status().as_u16(), 401);
}

// The route hands out a server-wide credential, so the guard is the whole boundary.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn minting_requires_an_admin_certificate() {
    let env = TestServer::start().await.unwrap();
    let resp = env
        .noauth_client()
        .unwrap()
        .post(format!("{}/api/admin/token", env.base_url))
        .send()
        .await
        .unwrap();
    HttpAssert::status(resp.status().as_u16(), 401);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_minted_token_appears_in_the_listing_and_can_be_revoked() {
    let env = TestServer::start().await.unwrap();
    let client = env.admin_client().unwrap();

    let minted: serde_json::Value = client
        .post(format!("{}/api/admin/token", env.base_url))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let id = minted["id"].as_str().unwrap().to_string();
    assert!(minted["token"].as_str().unwrap().starts_with("bvc_"));

    let listed: serde_json::Value = client
        .get(format!("{}/api/admin/token", env.base_url))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(
        listed["tokens"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["id"] == id.as_str())
    );

    let resp = client
        .delete(format!("{}/api/admin/token/{}", env.base_url, id))
        .send()
        .await
        .unwrap();
    HttpAssert::status(resp.status().as_u16(), 200);

    let listed: serde_json::Value = client
        .get(format!("{}/api/admin/token", env.base_url))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let row = listed["tokens"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == id.as_str())
        .unwrap()
        .clone();
    assert!(!row["revoked_at"].is_null());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rotating_returns_a_new_token_and_names_the_one_it_retired() {
    let env = TestServer::start().await.unwrap();
    let client = env.admin_client().unwrap();

    let original: serde_json::Value = client
        .post(format!("{}/api/admin/token", env.base_url))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let original_id = original["id"].as_str().unwrap().to_string();

    let rotated: serde_json::Value = client
        .post(format!(
            "{}/api/admin/token/{}/rotate",
            env.base_url, original_id
        ))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_ne!(rotated["id"].as_str().unwrap(), original_id);
    assert_eq!(rotated["revoked"].as_str().unwrap(), original_id);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rotating_an_unknown_id_is_not_found() {
    let env = TestServer::start().await.unwrap();
    let resp = env
        .admin_client()
        .unwrap()
        .post(format!("{}/api/admin/token/AbCdEfGh/rotate", env.base_url))
        .send()
        .await
        .unwrap();
    HttpAssert::status(resp.status().as_u16(), 404);
}

// The harness configures its token, so `configured` must be true and revoking it must be
// refused rather than silently doing nothing a restart would undo.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_legacy_route_reports_a_configured_scalar() {
    let env = TestServer::start().await.unwrap();
    let body: serde_json::Value = env
        .admin_client()
        .unwrap()
        .get(format!("{}/api/admin/token/legacy", env.base_url))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(body["token"].as_str().unwrap(), "test-mc-token");
    assert!(body["configured"].as_bool().unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn revoking_a_configured_legacy_scalar_is_refused() {
    let env = TestServer::start().await.unwrap();
    let resp = env
        .admin_client()
        .unwrap()
        .delete(format!("{}/api/admin/token/legacy", env.base_url))
        .send()
        .await
        .unwrap();
    HttpAssert::status(resp.status().as_u16(), 409);
}
