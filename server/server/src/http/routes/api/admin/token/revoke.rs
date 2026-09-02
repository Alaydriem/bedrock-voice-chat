use std::sync::Arc;

use rocket::{State, http::Status};
use rocket_okapi::openapi;

use crate::http::guards::AdminGuard;
use crate::services::{AccessTokenError, AccessTokenService};

/// Retire a game access token. The reserved id `legacy` removes the pre-identifier scalar.
#[openapi(tag = "Admin")]
#[delete("/token/<id>")]
pub async fn revoke_token(
    _admin: AdminGuard,
    id: &str,
    service: &State<Arc<AccessTokenService>>,
) -> Result<Status, Status> {
    match service.revoke(id).await {
        Ok(true) => Ok(Status::Ok),
        Ok(false) => Err(Status::NotFound),
        Err(AccessTokenError::LegacyIsConfigured) => Err(Status::Conflict),
        Err(_) => Err(Status::InternalServerError),
    }
}
