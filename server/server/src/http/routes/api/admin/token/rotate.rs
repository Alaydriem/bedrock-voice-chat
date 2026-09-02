use std::sync::Arc;

use common::response::admin::MintedTokenResponse;
use rocket::{State, http::Status, serde::json::Json};
use rocket_okapi::openapi;

use crate::http::guards::AdminGuard;
use crate::services::{AccessTokenError, AccessTokenService};

/// Issue a replacement and retire `id` in one transaction.
#[openapi(tag = "Admin")]
#[post("/token/<id>/rotate")]
pub async fn rotate_token(
    _admin: AdminGuard,
    id: &str,
    service: &State<Arc<AccessTokenService>>,
) -> Result<Json<MintedTokenResponse>, Status> {
    match service.rotate(id).await {
        Ok(minted) => Ok(Json(minted)),
        Err(AccessTokenError::UnknownId(_)) => Err(Status::NotFound),
        Err(_) => Err(Status::InternalServerError),
    }
}
