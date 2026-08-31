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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn config_reports_no_capacity_limit_by_default() {
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
    assert_eq!(
        body["capacity"]["limit"], 0,
        "a server with no limit configured must report unlimited"
    );
    assert_eq!(body["capacity"]["in_use"], 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn config_reports_the_configured_capacity_limit() {
    let env = TestServer::start_with_capacity(12).await.unwrap();
    let resp = env
        .noauth_client()
        .unwrap()
        .get(format!("{}/api/config", env.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["capacity"]["limit"], 12);
}

// A server that does not peer advertises nothing. The value grants no access, but it does
// name where an attacker would spend the unauthorized-connection budget, and a server with
// peering off has no use for it.
//
// The present case is not covered here: this harness always manages `None::<Arc<PeerPlane>>`,
// so standing one up would mean binding a real UDP endpoint for an assertion that
// `ticket_observed` already carries in `bvc-relay`'s own `peer::advertise` tests.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn config_omits_the_peer_link_when_no_peer_plane_is_bound() {
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
    assert!(body["peer_link"].is_null(), "got: {body}");
}
