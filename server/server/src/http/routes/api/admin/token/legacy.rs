use std::sync::Arc;

use common::response::admin::LegacyTokenResponse;
use rocket::{State, serde::json::Json};
use rocket_okapi::openapi;

use crate::http::guards::AdminGuard;
use crate::services::AccessTokenService;

/// The pre-identifier scalar, if this deployment still has one.
#[openapi(tag = "Admin")]
#[get("/token/legacy")]
pub async fn legacy_token(
    _admin: AdminGuard,
    service: &State<Arc<AccessTokenService>>,
) -> Json<LegacyTokenResponse> {
    Json(service.legacy())
}
