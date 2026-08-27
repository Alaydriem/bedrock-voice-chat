use std::sync::Arc;

use axum::routing::get;
use tower_http::cors::{AllowOrigin, CorsLayer};

use super::claim::ClaimRoutes;
use super::health::HealthRoutes;
use super::oauth::OAuthRoutes;
use super::state::HttpState;

pub struct Router;

impl Router {
    pub fn build(state: Arc<HttpState>) -> axum::Router {
        // CORS on the claim route alone. Everything else here is browser navigation
        // and needs none: adding it would advertise a cross-origin surface that does
        // not exist.
        //
        // A matched list rather than a fixed header. A fixed one is emitted to every
        // caller, which reads as a cross-origin grant to anyone inspecting the
        // response even though no browser would honour it.
        let cors = CorsLayer::new().allow_origin(AllowOrigin::list([state
            .http
            .page_origin
            .parse::<axum::http::HeaderValue>()
            .expect("page_origin is a valid header value")]));

        let claim = axum::Router::new()
            .route("/api/claim/{id}", get(ClaimRoutes::redeem))
            .layer(cors);

        axum::Router::new()
            .route("/oauth/start", get(OAuthRoutes::start))
            .route("/oauth/callback", get(OAuthRoutes::callback))
            .route("/healthz", get(HealthRoutes::healthz))
            .merge(claim)
            .with_state(state)
    }
}
