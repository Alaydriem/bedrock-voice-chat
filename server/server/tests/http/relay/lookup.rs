//! Relay routes are absent when `features.relay.enabled = false`.

use crate::harness::TestServer;

use common::structs::relay::{RegisterRequest, RelayEndpoint};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn routes_absent_when_relay_disabled() {
    let env = TestServer::start().await.unwrap();
    let client = env.noauth_client().unwrap();

    let body = RegisterRequest {
        hashed_world: "hW".to_string(),
        endpoint: RelayEndpoint {
            host: "a.example.com".to_string(),
            port: 1,
            primary: false,
        },
        ttl_secs: 60,
        token: String::new(),
    };
    let resp = client
        .post(format!("{}/relay/register", env.base_url))
        .json(&body)
        .send()
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        reqwest::StatusCode::NOT_FOUND,
        "relay route should not be mounted when disabled"
    );
}
