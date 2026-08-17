use common::response::auth::IntrospectResponse;
use rocket::{State, http::Status, mtls::Certificate, serde::json::Json};
use rocket_okapi::openapi;

use crate::config::Permissions;
use crate::http::guards::PlayerGuard;
use crate::http::pool::Db;
use crate::services::PermissionService;

/// Return the calling identity and its effective permissions.
///
/// Backs `bvc whoami`. Reports the identity, certificate expiry, and the permissions
/// in force after overrides are applied to the defaults.
///
/// Behind `PlayerGuard` like every other player route, so a banished or revoked caller is
/// refused here too. An earlier iteration left this open so a locked-out player could read
/// why; the decision is that they cannot.
#[openapi(tag = "Authentication")]
#[get("/auth/introspect")]
pub async fn introspect(
    guard: PlayerGuard,
    // Taken alongside the guard because the response reports the presented certificate's own
    // expiry, which the resolved player row does not carry.
    cert: Certificate<'_>,
    db: Db<'_>,
    perm_config: &State<Permissions>,
) -> Result<Json<IntrospectResponse>, Status> {
    let conn = db.into_inner();

    let player = guard.player;

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
