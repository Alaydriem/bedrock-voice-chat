use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use bvc_relay_service::config::{DiscordConfig, HttpConfig};
use bvc_relay_service::db::Db;
use bvc_relay_service::discord::{FixedMemberSource, IdentitySource, MemberSource};
use bvc_relay_service::http::{HttpState, Router, STATE_COOKIE};
use bvc_relay_service::registry::{ClaimService, RegistryService};
use tower::ServiceExt;

fn discord_config() -> DiscordConfig {
    DiscordConfig {
        guild_id: "guild".to_string(),
        bot_token: "bot".to_string(),
        client_id: "client".to_string(),
        client_secret: "secret".to_string(),
        qualifying_role_ids: vec!["role-a".to_string()],
    }
}

fn http_config() -> HttpConfig {
    HttpConfig {
        hostname: "registry.example".to_string(),
        page_origin: "https://page.example".to_string(),
        port: 443,
        bind: "::".to_string(),
        acme: Default::default(),
    }
}

async fn router(roles: Vec<String>) -> axum::Router {
    let conn = Arc::new(Db::connect("sqlite::memory:").await.expect("connects"));
    let registry = RegistryService::new_shared(
        conn.clone(),
        discord_config(),
        MemberSource::Fixed(FixedMemberSource::new(roles)),
    );
    let state = HttpState::new_shared(
        http_config(),
        discord_config(),
        registry,
        ClaimService::new_shared(conn),
        IdentitySource::Fixed("member-1".to_string()),
    );
    Router::build(state)
}

async fn get(app: &axum::Router, uri: &str, cookie: Option<&str>) -> axum::response::Response {
    let mut request = Request::builder().uri(uri);
    if let Some(cookie) = cookie {
        request = request.header("cookie", cookie);
    }
    app.clone()
        .oneshot(request.body(Body::empty()).expect("request builds"))
        .await
        .expect("response")
}

// The state cookie is the CSRF guard. It is set on this origin, compared at the
// callback, and without it anyone could feed the callback a code of their choosing.
#[tokio::test]
async fn starting_the_flow_sets_a_state_cookie_matching_the_redirect() {
    let app = router(vec!["role-a".to_string()]).await;

    let response = get(&app, "/oauth/start", None).await;

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let cookie = response
        .headers()
        .get("set-cookie")
        .expect("a state cookie is set")
        .to_str()
        .expect("ascii");
    assert!(cookie.starts_with(STATE_COOKIE));
    assert!(cookie.contains("HttpOnly"));
    assert!(cookie.contains("Secure"));

    let location = response
        .headers()
        .get("location")
        .expect("a redirect")
        .to_str()
        .expect("ascii");
    assert!(location.starts_with("https://discord.com/oauth2/authorize?"));
}

// A callback whose state does not match the cookie is refused. This is the one route
// where a missing check would be exploitable.
#[tokio::test]
async fn a_callback_with_a_mismatched_state_is_refused() {
    let app = router(vec!["role-a".to_string()]).await;

    let response = get(
        &app,
        "/oauth/callback?code=abc&state=attacker",
        Some(&format!("{STATE_COOKIE}=genuine")),
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn a_callback_with_no_cookie_at_all_is_refused() {
    let app = router(vec!["role-a".to_string()]).await;

    let response = get(&app, "/oauth/callback?code=abc&state=anything", None).await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// An entitled member is redirected to the page with a claim to redeem.
#[tokio::test]
async fn an_entitled_member_is_redirected_with_a_claim() {
    let app = router(vec!["role-a".to_string()]).await;

    let response = get(
        &app,
        "/oauth/callback?code=abc&state=s1",
        Some(&format!("{STATE_COOKIE}=s1")),
    )
    .await;

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let location = response
        .headers()
        .get("location")
        .expect("a redirect")
        .to_str()
        .expect("ascii");
    assert!(location.starts_with("https://page.example/enrolled?claim="));
}

// The failure that matters most: a member without a qualifying role must not be
// issued anything. They are redirected with a reason rather than shown a blank page.
#[tokio::test]
async fn an_unentitled_member_gets_a_reason_and_no_claim() {
    let app = router(vec!["role-z".to_string()]).await;

    let response = get(
        &app,
        "/oauth/callback?code=abc&state=s1",
        Some(&format!("{STATE_COOKIE}=s1")),
    )
    .await;

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let location = response
        .headers()
        .get("location")
        .expect("a redirect")
        .to_str()
        .expect("ascii");
    assert_eq!(location, "https://page.example/enrolled?error=not_entitled");
    assert!(!location.contains("claim="));
}

// Browser navigation carries no CORS headers. Only the claim API is called by script.
#[tokio::test]
async fn the_callback_carries_no_cors_headers() {
    let app = router(vec!["role-a".to_string()]).await;

    let response = get(
        &app,
        "/oauth/callback?code=abc&state=s1",
        Some(&format!("{STATE_COOKIE}=s1")),
    )
    .await;

    assert!(
        response
            .headers()
            .get("access-control-allow-origin")
            .is_none()
    );
}
