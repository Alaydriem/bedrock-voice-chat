use std::sync::Arc;

use entity::player;
use rocket::{
    serde::json::Json,
    State,
};
use sea_orm::{ActiveModelTrait, ActiveValue};
use sea_orm_rocket::Connection as SeaOrmConnection;

use common::response::auth::AuthStateResponse;
use common::response::error::ApiError;
use common::structs::permission::ServerPermissions;

use crate::config::ApplicationConfigPermissions;
use crate::rs::pool::AppDb;
use crate::services::{AuthService, CertificateService, PermissionService};
use crate::stream::quic::CacheManager;
use super::{GameHint, MtlsIdentity, RocketApiError};

#[get("/auth/state")]
pub async fn auth_state(
    identity: MtlsIdentity<'_>,
    game_hint: GameHint,
    db: SeaOrmConnection<'_, AppDb>,
    cert_service: &State<Arc<CertificateService>>,
    perm_config: &State<ApplicationConfigPermissions>,
    cache_manager: &State<CacheManager>,
) -> Result<Json<AuthStateResponse>, RocketApiError> {
    let conn = db.into_inner();

    let player_model =
        AuthService::player_from_certificate(&identity.0, conn, game_hint.0.as_deref())
            .await
            .map_err(|_| RocketApiError::from(ApiError::AuthFailed))?;

    // Clean stale channel memberships from any previous session
    if let Some(ref gamertag) = player_model.gamertag {
        if let Err(e) = cache_manager.remove_player(gamertag).await {
            tracing::error!("Failed to clean stale memberships for {}: {}", gamertag, e);
        }
    }

    let perm_service = PermissionService::new(perm_config.defaults.clone());
    let allowed = perm_service.evaluate_all(conn, player_model.id).await;

    let (certificate, certificate_key) = if player_model.needs_certificate_reissue() {
        let gamertag = player_model
            .gamertag
            .clone()
            .unwrap_or_else(|| "unknown".to_string());

        let game = player_model.game.clone();
        match cert_service.sign_player_cert(&gamertag, &game) {
            Ok((cert, key)) => {
                let cert_pem = cert.pem();
                let key_pem = key.serialize_pem();

                let mut active: player::ActiveModel = player_model.into();
                active.certificate = ActiveValue::Set(cert_pem.clone());
                active.certificate_key = ActiveValue::Set(key_pem.clone());
                if let Err(e) = active.update(conn).await {
                    tracing::error!("Failed to update player certificate: {}", e);
                    (None, None)
                } else {
                    tracing::info!("Re-issued certificate for player {}", gamertag);
                    (Some(cert_pem), Some(key_pem))
                }
            }
            Err(e) => {
                tracing::error!("Failed to re-issue certificate: {}", e);
                (None, None)
            }
        }
    } else {
        (None, None)
    };

    Ok(Json(AuthStateResponse {
        server_permissions: ServerPermissions { allowed },
        certificate,
        certificate_key,
    }))
}
