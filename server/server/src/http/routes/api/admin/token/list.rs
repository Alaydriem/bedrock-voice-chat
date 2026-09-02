use std::sync::Arc;

use common::response::admin::{AccessTokenListResponse, AccessTokenRow};
use rocket::{State, http::Status, serde::json::Json};
use rocket_okapi::openapi;

use crate::http::guards::AdminGuard;
use crate::http::pool::Db;
use crate::services::AccessTokenService;

/// List every issued game access token. Secrets are never returned.
#[openapi(tag = "Admin")]
#[get("/token")]
pub async fn list_tokens(
    _admin: AdminGuard,
    db: Db<'_>,
    service: &State<Arc<AccessTokenService>>,
) -> Result<Json<AccessTokenListResponse>, Status> {
    let mut tokens = AccessTokenService::list_in(db.into_inner())
        .await
        .map_err(|_| Status::InternalServerError)?;

    // The pre-identifier scalar is a live credential, so it belongs in the listing under
    // its reserved id or an operator cannot see that it is still accepted.
    if service.legacy().token.is_some() {
        tokens.insert(
            0,
            AccessTokenRow {
                id: AccessTokenService::LEGACY_ID.to_string(),
                created_at: 0,
                revoked_at: None,
            },
        );
    }

    Ok(Json(AccessTokenListResponse { tokens }))
}
