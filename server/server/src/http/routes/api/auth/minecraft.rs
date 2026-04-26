use std::sync::Arc;

use common::{
    auth::{AuthError as CommonAuthError, MinecraftAuthenticator},
    request::LoginRequest,
    response::LoginResponse,
    Game,
};
use rocket::{http::Status, serde::json::Json, State};
use rocket_okapi::openapi;

use crate::config::{Permissions, Server};
use crate::http::dtos::ncryptf::JsonMessage;
use crate::http::openapi::NcryptfJsonResponse;
use crate::http::pool::Db;
use crate::services::{AuthError, AuthService, CertificateService, PermissionService, PlayerIdentityService, PlayerRegistrarService};

/// Authenticates the Player via Xbox Live to grab their gamertag and other identifying information
#[openapi(tag = "Authentication")]
#[post("/auth/minecraft", data = "<payload>")]
pub async fn authenticate(
    db: Db<'_>,
    payload: Json<LoginRequest>,
    config: &State<Server>,
    cert_service: &State<Arc<CertificateService>>,
    identity_service: &State<PlayerIdentityService>,
    player_registrar: &State<PlayerRegistrarService>,
    perm_config: &State<Permissions>,
    authenticator: &State<Arc<dyn MinecraftAuthenticator>>,
) -> NcryptfJsonResponse<LoginResponse> {
    let conn = db.into_inner();

    let code = payload.0.code;
    let redirect_uri = match payload.0.redirect_uri.parse() {
        Ok(uri) => uri,
        Err(e) => {
            tracing::error!("Invalid redirect URI: {}", e);
            return NcryptfJsonResponse::from_inner(JsonMessage::create(Status::BadRequest, None, None, None));
        }
    };

    let auth_result = match authenticator.authenticate(code, redirect_uri).await {
        Ok(result) => result,
        Err(e) => {
            tracing::error!("Xbox Live authentication failed: {}", e);
            return match e {
                CommonAuthError::ProfileNotFound => {
                    NcryptfJsonResponse::from_inner(JsonMessage::create(Status::Forbidden, None, None, None))
                }
                _ => NcryptfJsonResponse::from_inner(JsonMessage::create(Status::Forbidden, None, None, None)),
            };
        }
    };

    let gamertag = auth_result.gamertag.clone();
    let minecraft_username = auth_result.minecraft_username.clone();
    let mc_uuid = auth_result.minecraft_uuid.clone();

    let xbl_player_id = identity_service
        .find_player_id_by_gamertag(&gamertag, &Game::Minecraft)
        .await;

    let uuid_player_id = match mc_uuid.as_deref() {
        Some(uuid) => {
            identity_service
                .find_player_id_by_alias(uuid, "platform_uuid", &Game::Minecraft)
                .await
        }
        None => None,
    };

    // Phase B re-mapping: create XBL record when Java UUID alias resolves
    if xbl_player_id.is_none() && uuid_player_id.is_some() {
        player_registrar
            .create_player(&gamertag, &Game::Minecraft, None)
            .await;
    }

    let perm_service = PermissionService::new(perm_config.defaults.clone());
    match AuthService::build_login_response(
        conn,
        config.inner(),
        &cert_service,
        Some(&perm_service),
        gamertag.clone(),
        auth_result.gamerpic,
        Game::Minecraft,
    )
    .await
    {
        Ok(mut response) => {
            if minecraft_username.is_some() || mc_uuid.is_some() {
                if let Some(player_id) = identity_service
                    .find_player_id_by_gamertag(&gamertag, &Game::Minecraft)
                    .await
                {
                    if let Some(ref mc_name) = minecraft_username {
                        if mc_name != &gamertag {
                            if let Err(e) = identity_service
                                .create_alias(
                                    player_id,
                                    mc_name,
                                    &Game::Minecraft,
                                    "minecraft_services",
                                )
                                .await
                            {
                                tracing::warn!(
                                    "Failed to create minecraft_services alias for {}: {}",
                                    mc_name,
                                    e
                                );
                            }
                        }
                    }
                    if let Some(ref uuid) = mc_uuid {
                        if let Err(e) = identity_service
                            .create_alias(player_id, uuid, &Game::Minecraft, "platform_uuid")
                            .await
                        {
                            tracing::warn!(
                                "Failed to create platform_uuid alias for {}: {}",
                                uuid,
                                e
                            );
                        }
                    }
                }
            }

            response.minecraft_username = minecraft_username;

            NcryptfJsonResponse::from_inner(JsonMessage::create(Status::Ok, Some(response), None, None))
        }
        Err(e) => {
            tracing::error!("Login failed: {}", e);
            match e {
                AuthError::PlayerNotFound | AuthError::PlayerBanished => {
                    NcryptfJsonResponse::from_inner(JsonMessage::create(Status::Forbidden, None, None, None))
                }
                _ => NcryptfJsonResponse::from_inner(JsonMessage::create(Status::InternalServerError, None, None, None)),
            }
        }
    }
}
