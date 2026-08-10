//! Rate limiting on the unauthenticated relay code endpoints.
//!
//! The limiter is keyed on what each request is about — the world an offer injects into,
//! the presenter a redemption names — rather than on the caller's address. Behind the TLS
//! demultiplexer every caller arrives from loopback, so an address-keyed limiter would
//! hold one global bucket and a single busy peer would throttle the whole mesh.

use crate::harness::TestServer;

use bvc_server_lib::services::RelayRateLimiter;
use common::structs::relay::OfferRequest;

fn offer_for(world: &str) -> OfferRequest {
    OfferRequest {
        hashed_world: world.to_string(),
        asker_host: "asker.example.com".to_string(),
        asker_port: 6000,
        asker_public_key: vec![7u8; 32],
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn flooding_one_world_is_throttled() {
    let env = TestServer::start_with_relay(true).await.unwrap();
    let client = env.noauth_client().unwrap();
    let body = offer_for("world-under-flood");

    let mut statuses = Vec::new();
    for _ in 0..(RelayRateLimiter::PER_MINUTE + 1) {
        let resp = client
            .post(format!("{}/api/relay/offer", env.base_url))
            .json(&body)
            .send()
            .await
            .unwrap();
        statuses.push(resp.status());
    }

    assert!(
        statuses
            .iter()
            .any(|s| *s == reqwest::StatusCode::TOO_MANY_REQUESTS),
        "exceeding the per-world offer quota must yield 429; saw {statuses:?}"
    );
}

/// The point of the rekey. Both callers share one source address — they are the same
/// process — so an address-keyed limiter would have drained a single bucket and throttled
/// the second world along with the first.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn one_flooded_world_does_not_throttle_another() {
    let env = TestServer::start_with_relay(true).await.unwrap();
    let client = env.noauth_client().unwrap();

    let flooded = offer_for("world-under-flood");
    for _ in 0..(RelayRateLimiter::PER_MINUTE + 1) {
        let _ = client
            .post(format!("{}/api/relay/offer", env.base_url))
            .json(&flooded)
            .send()
            .await
            .unwrap();
    }

    let bystander = offer_for("world-minding-its-own-business");
    let resp = client
        .post(format!("{}/api/relay/offer", env.base_url))
        .json(&bystander)
        .send()
        .await
        .unwrap();

    assert_ne!(
        resp.status(),
        reqwest::StatusCode::TOO_MANY_REQUESTS,
        "a world that sent nothing must keep its own quota"
    );
}
