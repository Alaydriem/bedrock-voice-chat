use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use common::curia;
use serde::Serialize;

use super::state::HttpState;

#[derive(Serialize)]
pub struct ClaimResponse {
    pub token: String,
}

pub struct ClaimRoutes;

impl ClaimRoutes {
    // Unknown, expired and already-redeemed all answer 404. The page cannot act on
    // the difference, and naming it would tell a guesser which ids exist.
    pub async fn redeem(State(state): State<Arc<HttpState>>, Path(id): Path<String>) -> Response {
        match state.claims.redeem(&id).await {
            Ok(Some(token)) => (StatusCode::OK, Json(ClaimResponse { token })).into_response(),
            Ok(None) => StatusCode::NOT_FOUND.into_response(),
            Err(e) => {
                curia::error!(format!("redeeming a claim failed: {e}"));
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}
