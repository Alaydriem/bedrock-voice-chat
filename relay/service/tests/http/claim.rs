use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use bvc_relay_service::config::{DiscordConfig, HttpConfig};
use bvc_relay_service::db::Db;
use bvc_relay_service::discord::{FixedMemberSource, IdentitySource, MemberSource};
use bvc_relay_service::http::{HttpState, Router};
use bvc_relay_service::registry::{ClaimService, RegistryService};
use tower::ServiceExt;

async fn app_with_claim() -> (axum::Router, String) {
    let conn = Arc::new(Db::connect("sqlite::memory:").await.expect("connects"));
    let discord = DiscordConfig {
        guild_id: "guild".to_string(),
        bot_token: "bot".to_string(),
        client_id: "client".to_string(),
        client_secret: "secret".to_string(),
        qualifying_role_ids: vec!["role-a".to_string()],
    };
    let registry = RegistryService::new_shared(
        conn.clone(),
        discord.clone(),
        MemberSource::Fixed(FixedMemberSource::new(vec!["role-a".to_string()])),
    );
    let claims = ClaimService::new_shared(conn);
    let id = claims.store("bvcenroll-abc").await.expect("stores");

    let state = HttpState::new_shared(
        HttpConfig {
            hostname: "registry.example".to_string(),
            page_origin: "https://page.example".to_string(),
            port: 443,
            bind: "::".to_string(),
            acme: Default::default(),
        },
        discord,
        registry,
        claims,
        IdentitySource::Fixed("member-1".to_string()),
    );
    (Router::build(state), id)
}

async fn get_with_origin(app: &axum::Router, uri: &str, origin: &str) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .uri(uri)
                .header("origin", origin)
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("response")
}

// The claim is the only route script calls, so it is the only one that answers a
// cross-origin request — and it answers exactly one origin.
#[tokio::test]
async fn the_configured_page_origin_is_allowed() {
    let (app, id) = app_with_claim().await;

    let response = get_with_origin(&app, &format!("/api/claim/{id}"), "https://page.example").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("access-control-allow-origin")
            .expect("an allow-origin header")
            .to_str()
            .expect("ascii"),
        "https://page.example"
    );
}

// Any other origin is refused the headers it would need to read the body. An
// enrollment token readable by any page is a token anyone can take.
#[tokio::test]
async fn another_origin_is_not_allowed() {
    let (app, id) = app_with_claim().await;

    let response = get_with_origin(&app, &format!("/api/claim/{id}"), "https://evil.example").await;

    assert!(
        response
            .headers()
            .get("access-control-allow-origin")
            .is_none()
    );
}

// Redemption is once. A replayable claim is a token anyone who saw the URL can take.
#[tokio::test]
async fn a_claim_redeems_once_and_is_then_gone() {
    let (app, id) = app_with_claim().await;

    let first = get_with_origin(&app, &format!("/api/claim/{id}"), "https://page.example").await;
    assert_eq!(first.status(), StatusCode::OK);

    let second = get_with_origin(&app, &format!("/api/claim/{id}"), "https://page.example").await;
    assert_eq!(second.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn an_unknown_claim_is_not_found() {
    let (app, _) = app_with_claim().await;

    let response = get_with_origin(&app, "/api/claim/nonexistent", "https://page.example").await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
