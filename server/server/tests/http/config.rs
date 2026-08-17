//! GET /api/config
//!
//! Contract:
//! - unauthenticated, and the usual way to confirm a server is reachable
//! - `chat.enabled` mirrors `server.features.chat`, defaulting to true

use crate::harness::TestServer;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn config_reports_chat_enabled_by_default() {
    let env = TestServer::start().await.unwrap();
    let resp = env
        .noauth_client()
        .unwrap()
        .get(format!("{}/api/config", env.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["chat"]["enabled"], true);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn config_reports_chat_disabled_when_the_operator_turns_it_off() {
    let env = TestServer::start_with_chat(false).await.unwrap();
    let resp = env
        .noauth_client()
        .unwrap()
        .get(format!("{}/api/config", env.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["chat"]["enabled"], false);
}
