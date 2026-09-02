use std::sync::Arc;

use common::response::admin::MintedTokenResponse;
use rocket::{State, http::Status, serde::json::Json};
use rocket_okapi::openapi;

use crate::http::guards::AdminGuard;
use crate::services::AccessTokenService;

/// Issue a game access token. The secret is returned once and is not recoverable.
#[openapi(tag = "Admin")]
#[post("/token")]
pub async fn mint_token(
    _admin: AdminGuard,
    service: &State<Arc<AccessTokenService>>,
) -> Result<Json<MintedTokenResponse>, Status> {
    service
        .mint()
        .await
        .map(Json)
        .map_err(|_| Status::InternalServerError)
}
