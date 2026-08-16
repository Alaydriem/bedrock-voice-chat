//! GET /api/admin/relay/worlds
//!
//! Contract:
//! - 401 admin gate
//! - 200 with an empty list when no player is in a relay world

use crate::harness::{HttpAssert, TestServer};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn returns_401_without_client_cert() {
    let env = TestServer::start().await.unwrap();
    let resp = env
        .noauth_client()
        .unwrap()
        .get(format!("{}/api/admin/relay/worlds", env.base_url))
        .send()
        .await
        .unwrap();
    HttpAssert::status(resp.status().as_u16(), 401);
}

// An empty list is the honest answer for a server nobody is connected to, and it
// is distinct from the 404 the peer link route gives when peering is off.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn returns_an_empty_list_when_no_player_is_in_a_world() {
    let env = TestServer::start().await.unwrap();
    let resp = env
        .admin_client()
        .unwrap()
        .get(format!("{}/api/admin/relay/worlds", env.base_url))
        .send()
        .await
        .unwrap();
    HttpAssert::status(resp.status().as_u16(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["worlds"].as_array().unwrap().len(), 0);
}
