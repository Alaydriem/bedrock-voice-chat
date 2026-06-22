use common::response::auth::IntrospectResponse;
use rocket::{State, http::Status, mtls::Certificate, serde::json::Json};
use rocket_okapi::openapi;

use crate::config::Permissions;
use crate::http::pool::Db;
use crate::services::{AuthService, PermissionService};

#[openapi(tag = "Authentication")]
#[get("/auth/introspect")]
pub async fn introspect(
    cert: Certificate<'_>,
    db: Db<'_>,
    perm_config: &State<Permissions>,
) -> Result<Json<IntrospectResponse>, Status> {
    let conn = db.into_inner();

    let player = AuthService::player_from_certificate(&cert, conn, None).await?;

    let perm_service = PermissionService::new(perm_config.defaults.clone());
    let permissions = perm_service.evaluate_all(conn, player.id).await;

    let cert_not_after = cert.validity().not_after.timestamp();

    let gamertag = player
        .gamertag
        .clone()
        .unwrap_or_else(|| "<unknown>".to_string());

    Ok(Json(IntrospectResponse {
        gamertag,
        game: player.game,
        cert_not_after: Some(cert_not_after),
        permissions,
    }))
}
