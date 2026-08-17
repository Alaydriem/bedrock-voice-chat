//! GET /api/clients/live
//!
//! Contract:
//! - gated by the game access token, like every other route the mod calls
//! - returns the identities with a live voice connection, so a bridge can leave
//!   them out of its own injection

use crate::harness::TestServer;

const TOKEN: &str = "test-mc-token";

// The SVC bridge asks so it can suppress locally. Unauthenticated it would be a
// roster of who is currently talking, readable by anyone who can reach the port.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn live_clients_rejects_a_missing_token() {
    let env = TestServer::start().await.unwrap();

    let resp = env
        .noauth_client()
        .unwrap()
        .get(format!("{}/api/clients/live", env.base_url))
        .send()
        .await
        .unwrap();

    assert_ne!(
        resp.status(),
        200,
        "a request without the game token must be rejected"
    );
}

// Empty rather than absent when nobody is connected. A bridge reads this as
// "suppress nobody", which is the correct behaviour on a server whose players have
// not opened a voice client — and it must not be confused with a failure, where
// suppressing nobody is also what happens but for the wrong reason.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn live_clients_is_empty_when_nobody_is_connected() {
    let env = TestServer::start().await.unwrap();

    let resp = env
        .noauth_client()
        .unwrap()
        .get(format!("{}/api/clients/live", env.base_url))
        .header("Authorization", format!("Bearer {TOKEN}"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status().as_u16(), 200);

    let body: Vec<String> = resp.json().await.unwrap();
    assert!(body.is_empty(), "expected no live clients, got {:?}", body);
}
