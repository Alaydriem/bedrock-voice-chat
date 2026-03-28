use std::sync::Arc;

use entity::player;
use rocket::{http::Status, mtls::Certificate, State};
use sea_orm::{ActiveModelTrait, ActiveValue};
use crate::http::openapi::CustomJsonResponse;
use common::response::auth::AuthStateResponse;
use common::structs::permission::ServerPermissions;
use rocket_okapi::openapi;

use crate::config::Permissions;
use crate::http::pool::Db;
use crate::services::{AuthService, CertificateService, PermissionService};
use crate::stream::quic::CacheManager;

#[openapi(tag = "Identity")]
#[get("/auth/state")]
pub async fn auth_state(
    identity: Certificate<'_>,
    db: Db<'_>,
    cert_service: &State<Arc<CertificateService>>,
    perm_config: &State<Permissions>,
    cache_manager: &State<CacheManager>,
) -> CustomJsonResponse<AuthStateResponse> {
    let conn = db.into_inner();

    let player_model = match AuthService::player_from_certificate(&identity, conn, None).await {
        Ok(p) => p,
        Err(status) => return CustomJsonResponse::error(status),
    };

    // Clean stale channel memberships
    if let Some(ref gt) = player_model.gamertag {
        if let Err(e) = cache_manager.remove_player(gt).await {
            tracing::error!("Failed to clean stale memberships for {}: {}", gt, e);
        }
    }

    let perm_service = PermissionService::new(perm_config.defaults.clone());
    let allowed = perm_service.evaluate_all(conn, player_model.id).await;

    let (certificate, certificate_key) = match player_model.is_certificate_expiring() {
        Ok(true) => {
            let gt = player_model
                .gamertag
                .clone()
                .unwrap_or_else(|| "unknown".to_string());

            match cert_service.sign_player_cert(&gt) {
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
                        tracing::info!("Re-issued certificate for player {}", gt);
                        (Some(cert_pem), Some(key_pem))
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to re-issue certificate: {}", e);
                    (None, None)
                }
            }
        }
        _ => (None, None),
    };

    CustomJsonResponse::ok(AuthStateResponse {
        server_permissions: ServerPermissions { allowed },
        certificate,
        certificate_key,
    })
}
