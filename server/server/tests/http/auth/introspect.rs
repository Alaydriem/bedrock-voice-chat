//! GET /api/auth/introspect
//!
//! Contract:
//! - mTLS-required (no admin permission needed; any authenticated player can introspect themselves)
//! - 401 / 403 when no cert present (Certificate guard fails)
//! - 200 returning the player's own gamertag, game, cert NotAfter, and granted permissions
//! - 403 when the cert resolves to no player (untrusted cert / unknown gamertag)
//!
//! Introspect sits behind `PlayerGuard`, so a banished or revoked caller is refused here as
//! well. An earlier iteration left it open so a locked-out player could read why; that is no
//! longer the case. The guard's own tests cover both refusals.

use crate::harness::http_client::MtlsClient;
use crate::harness::server::ADMIN_GAMERTAG;
use crate::harness::{HttpAssert, TestServer};

use common::Game;

const ENDPOINT: &str = "/api/auth/introspect";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn returns_401_or_403_without_client_cert() {
    let env = TestServer::start().await.unwrap();
    let resp = env
        .noauth_client()
        .unwrap()
        .get(format!("{}{}", env.base_url, ENDPOINT))
        .send()
        .await
        .unwrap();
    let status = resp.status().as_u16();
    assert!(
        status == 401 || status == 403,
        "expected 401 or 403 for no-cert request, got {}",
        status
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn admin_introspects_self_with_admin_permission() {
    let env = TestServer::start().await.unwrap();
    let resp = env
        .admin_client()
        .unwrap()
        .get(format!("{}{}", env.base_url, ENDPOINT))
        .send()
        .await
        .unwrap();
    HttpAssert::status(resp.status().as_u16(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["gamertag"].as_str().unwrap(), ADMIN_GAMERTAG);
    assert_eq!(body["game"].as_str().unwrap(), "minecraft");
    assert!(body["cert_not_after"].is_number());

    let perms: Vec<String> = body["permissions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert!(
        perms.contains(&"admin".to_string()),
        "admin should hold the admin permission, got {:?}",
        perms
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn non_admin_introspects_self_with_empty_permissions() {
    let env = TestServer::start().await.unwrap();
    let (cert, key) = env.issue_player("Bob", &Game::Minecraft).await.unwrap();
    let resp = MtlsClient::with_identity(&env.ca_pem, &cert, &key)
        .unwrap()
        .get(format!("{}{}", env.base_url, ENDPOINT))
        .send()
        .await
        .unwrap();
    HttpAssert::status(resp.status().as_u16(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["gamertag"].as_str().unwrap(), "Bob");
    assert_eq!(body["permissions"].as_array().unwrap().len(), 0);
}
