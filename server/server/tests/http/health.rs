//! GET /health/liveness and GET /health/readiness
//!
//! Contract:
//! - both unauthenticated (K8s probes carry no client cert; mutual.mandatory = false)
//! - liveness: 200 whenever Rocket serves, no dependency checks
//! - readiness: 503 with per-component JSON until every component is ready,
//!   200 once the QUIC flag is set and the database pings

use crate::harness::TestServer;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn liveness_returns_200_unauthenticated() {
    let env = TestServer::start().await.unwrap();
    let resp = env
        .noauth_client()
        .unwrap()
        .get(format!("{}/health/liveness", env.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 200);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn readiness_reports_components_and_follows_quic_flag() {
    let env = TestServer::start().await.unwrap();
    let client = env.noauth_client().unwrap();
    let url = format!("{}/health/readiness", env.base_url);

    // The harness never starts QUIC, so readiness must be 503 with the quic
    // component down but the database component ok.
    let resp = client.get(&url).send().await.unwrap();
    assert_eq!(resp.status().as_u16(), 503);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["database"], "ok");
    assert_eq!(body["quic"], "down");
    assert_eq!(body["certificate"], "ok");

    env.readiness.set_quic_ready(true);
    let resp = client.get(&url).send().await.unwrap();
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["database"], "ok");
    assert_eq!(body["quic"], "ok");
    assert_eq!(body["certificate"], "ok");
}

// Before the relay has pushed a challenge there is nothing to echo. A 404 says that
// plainly; an empty 200 would read to the relay as a server answering with the wrong
// value rather than one that has not been asked.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_enrollment_nonce_is_absent_until_the_relay_sends_one() {
    let env = TestServer::start().await.unwrap();

    let resp = env
        .noauth_client()
        .unwrap()
        .get(format!("{}/health/enrollment-nonce", env.base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status().as_u16(), 404);
}

// The relay compares the body byte for byte against what it pushed, so the route
// echoes it verbatim and unauthenticated — the relay holds no credential for this
// server at the moment it probes the declared address.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_enrollment_nonce_is_echoed_verbatim_and_unauthenticated() {
    let env = TestServer::start().await.unwrap();
    env.nonce.set("abc123".to_string());

    let resp = env
        .noauth_client()
        .unwrap()
        .get(format!("{}/health/enrollment-nonce", env.base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status().as_u16(), 200);
    assert_eq!(resp.text().await.unwrap(), "abc123");
}
