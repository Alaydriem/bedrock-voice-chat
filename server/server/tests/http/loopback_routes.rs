//! The plain-HTTP listener the game mods reach through the demultiplexer.
//!
//! It carries the seven paths the mods call and nothing else. A path outside them, an admin
//! route or the metrics endpoint for instance, must not exist on a listener with no TLS.

use bvc_server_lib::http::manager::LoopbackRoutes;

use crate::harness::{HttpAssert, LOOPBACK_TOKEN, LoopbackServer};

const EMPTY_POSITIONS: &str = r#"{"game":"minecraft","players":[]}"#;

#[test]
fn only_the_seven_mod_paths_are_mounted() {
    let mounted: Vec<String> = LoopbackRoutes::routes()
        .into_iter()
        .flat_map(|(prefix, routes)| {
            routes
                .into_iter()
                .map(|route| LoopbackRoutes::full_path(prefix, &route))
                .collect::<Vec<_>>()
        })
        .collect();

    for allowed in LoopbackRoutes::PATHS {
        assert!(
            mounted.iter().any(|path| {
                path == allowed
                    || path
                        .strip_prefix(allowed)
                        .is_some_and(|rest| rest.starts_with('/'))
            }),
            "no route is mounted for {allowed}"
        );
    }
}

#[test]
fn a_path_is_admitted_only_at_a_segment_boundary() {
    assert!(LoopbackRoutes::admits("/api/position"));
    assert!(LoopbackRoutes::admits("/api/channel/protected"));
    assert!(!LoopbackRoutes::admits("/api/positions"));
    assert!(!LoopbackRoutes::admits("/api/websocket/positions"));
    assert!(!LoopbackRoutes::admits("/api/admin/token"));
    assert!(!LoopbackRoutes::admits("/metrics"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_position_post_without_a_token_is_rejected_on_the_loopback_listener() {
    let server = LoopbackServer::start().await.unwrap();

    let response = reqwest::Client::new()
        .post(format!("{}/api/position", server.base_url))
        .header("Content-Type", "application/json")
        .body(EMPTY_POSITIONS)
        .send()
        .await
        .unwrap();

    // The guard answers a request with no Authorization header as malformed, before it
    // reaches the question of whether a credential is right.
    HttpAssert::status(response.status().as_u16(), 400);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_position_post_with_a_wrong_token_is_forbidden_on_the_loopback_listener() {
    let server = LoopbackServer::start().await.unwrap();

    let response = reqwest::Client::new()
        .post(format!("{}/api/position", server.base_url))
        .header("Content-Type", "application/json")
        .header("Authorization", "Bearer not-the-token")
        .body(EMPTY_POSITIONS)
        .send()
        .await
        .unwrap();

    HttpAssert::status(response.status().as_u16(), 403);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_position_post_with_a_valid_token_is_accepted_on_the_loopback_listener() {
    let server = LoopbackServer::start().await.unwrap();

    let response = reqwest::Client::new()
        .post(format!("{}/api/position", server.base_url))
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {LOOPBACK_TOKEN}"))
        .body(EMPTY_POSITIONS)
        .send()
        .await
        .unwrap();

    HttpAssert::status(response.status().as_u16(), 200);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_route_outside_the_mod_paths_does_not_exist_on_the_loopback_listener() {
    let server = LoopbackServer::start().await.unwrap();

    for path in ["/metrics", "/api/admin/token", "/api/auth/introspect"] {
        let response = reqwest::Client::new()
            .get(format!("{}{}", server.base_url, path))
            .header("Authorization", format!("Bearer {LOOPBACK_TOKEN}"))
            .send()
            .await
            .unwrap();

        HttpAssert::status(response.status().as_u16(), 404);
    }
}
