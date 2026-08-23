use common::request::admin::BanishUserRequest;
use common::response::admin::BanishedUserResponse;
use entity::player;
use rocket::{http::Status, serde::json::Json};
use rocket_okapi::openapi;
use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter};

use crate::http::guards::AdminGuard;
use crate::http::pool::Db;

/// Toggle a player's banished flag.
///
/// Reversible, and retains the player record.
#[openapi(tag = "Admin")]
#[patch("/user/banish", data = "<payload>")]
pub async fn banish_user(
    admin: AdminGuard,
    db: Db<'_>,
    revocations: &rocket::State<std::sync::Arc<crate::services::CertificateRevocationService>>,
    cache_manager: &rocket::State<crate::stream::quic::CacheManager>,
    payload: Json<BanishUserRequest>,
) -> Result<Json<BanishedUserResponse>, Status> {
    let conn = db.into_inner();
    let req = payload.0;

    crate::http::routes::api::admin::validate_gamertag(&req.gamertag)?;

    let admin_gamertag = admin.player.gamertag.as_deref().unwrap_or_default();
    if admin_gamertag == req.gamertag && admin.player.game == req.game {
        tracing::warn!(
            "banish_user: rejecting self-banish attempt by {} ({:?})",
            admin_gamertag,
            admin.player.game,
        );
        return Err(Status::Conflict);
    }

    let player_record = player::Entity::find()
        .filter(player::Column::Gamertag.eq(req.gamertag.clone()))
        .filter(player::Column::Game.eq(req.game.clone()))
        .one(conn)
        .await
        .map_err(|e| {
            tracing::error!("banish_user: db error: {}", e);
            Status::InternalServerError
        })?
        .ok_or(Status::NotFound)?;

    let certificate = player_record.certificate.clone();
    let player_id = player_record.id;

    let mut active: player::ActiveModel = player_record.into();
    active.banished = ActiveValue::Set(req.banish);

    active.update(conn).await.map_err(|e| {
        tracing::error!("banish_user: update failed: {}", e);
        Status::InternalServerError
    })?;

    // Banning has to act on the certificate the player already holds. Setting the flag alone
    // took effect only at their next login, which a banned player has no reason to perform.
    //
    // A failure here is a 500, not a warning: a ban that wrote the flag but not the
    // revocation is precisely the defect this closes, and reporting success would hide it.
    if req.banish {
        revocations
            .revoke_pem(conn, &certificate, Some(player_id), "banished")
            .await
            .map_err(|e| {
                tracing::error!("banish_user: failed to revoke certificate: {}", e);
                Status::InternalServerError
            })?;

        if let Some(fingerprint) =
            common::structs::certificate::CertificateFingerprint::from_pem(&certificate)
            && let Some(registry) = cache_manager.get_connection_registry()
            && registry.revoke_session(&fingerprint, "Your access to this server has been revoked.")
        {
            tracing::info!(
                "banish_user: closed the live session held by {}",
                req.gamertag
            );
        }
    }

    // Unbanning deliberately does not un-revoke. The certificate is gone; the player logs in
    // and is issued a new one, which `banished` no longer blocks.

    Ok(Json(BanishedUserResponse {
        gamertag: req.gamertag,
        game: req.game,
        banished: req.banish,
    }))
}
