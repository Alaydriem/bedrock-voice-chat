//! GET /metrics
//!
//! Contract:
//! - unauthenticated (no Certificate guard) — reachable by a plain CA-trusting client
//!   over the mTLS listener (mutual.mandatory = false)
//! - 200 with a Prometheus text exposition (build info + TYPE lines)
//! - never leaks player identity (aggregate only)

use crate::harness::TestServer;

const ENDPOINT: &str = "/metrics";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn metrics_endpoint_serves_prometheus_exposition_unauthenticated() {
    let env = TestServer::start().await.unwrap();

    let resp = env
        .noauth_client()
        .unwrap()
        .get(format!("{}{}", env.base_url, ENDPOINT))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status().as_u16(), 200);

    let body = resp.text().await.unwrap();
    assert!(
        body.contains("bvc_build_info"),
        "expected build info in exposition, got:\n{body}"
    );
    assert!(body.contains("# TYPE"), "expected TYPE metadata, got:\n{body}");
    assert!(
        !body.to_lowercase().contains("gamertag"),
        "metrics must not leak player identity"
    );
}
