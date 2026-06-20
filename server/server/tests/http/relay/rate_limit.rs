//! Per-IP rate limiting (rocket_governor) on the unauthenticated relay code
//! endpoints. The limiter is a request guard that runs BEFORE the handler, so a
//! throttled request is rejected (429) regardless of payload validity.
//!
//! This floods `/api/relay/offer`, which holds its OWN per-IP bucket (a distinct
//! `RocketGovernable` type) — governor's limiter is process-global per type, so
//! flooding offer here cannot drain the peer-redeem bucket the other relay tests
//! rely on.

use crate::harness::TestServer;

use common::structs::relay::OfferRequest;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn offer_is_rate_limited_per_ip() {
    let env = TestServer::start_with_relay(true).await.unwrap();
    let client = env.noauth_client().unwrap();

    let body = OfferRequest {
        hashed_world: "hW".to_string(),
        asker_host: "asker.example.com".to_string(),
        asker_port: 6000,
        asker_public_key: vec![7u8; 32],
    };

    // The quota is 3/min/IP. A fresh burst allows 3; the 4th sequential request
    // from the same IP must be throttled.
    let mut statuses = Vec::new();
    for _ in 0..4 {
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
        "exceeding the per-IP offer quota must yield 429; saw {statuses:?}"
    );
}
