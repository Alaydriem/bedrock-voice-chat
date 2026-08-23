//! GET /api/admin/relay/peerlink
//!
//! Contract:
//! - 401 admin gate
//! - 404 when peering is not configured

use crate::harness::{HttpAssert, TestServer};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn returns_401_without_client_cert() {
    let env = TestServer::start().await.unwrap();
    let resp = env
        .noauth_client()
        .unwrap()
        .get(format!("{}/api/admin/relay/peerlink", env.base_url))
        .send()
        .await
        .unwrap();
    HttpAssert::status(resp.status().as_u16(), 401);
}

// A server with no `peer` block binds no peer endpoint, so it has no link to
// print. Answering 404 rather than an empty string is what lets the CLI say
// peering is off instead of printing nothing.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn returns_404_when_peering_is_not_configured() {
    let env = TestServer::start().await.unwrap();
    let resp = env
        .admin_client()
        .unwrap()
        .get(format!("{}/api/admin/relay/peerlink", env.base_url))
        .send()
        .await
        .unwrap();
    HttpAssert::status(resp.status().as_u16(), 404);
}
