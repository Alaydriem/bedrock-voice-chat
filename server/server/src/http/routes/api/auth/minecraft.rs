use common::{
    auth::{AuthError as CommonAuthError, MinecraftAuthProvider},
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
use crate::services::{AuthError, AuthService, PermissionService, PlayerIdentityService};

/// Authenticates the Player via Xbox Live to grab their gamertag and other identifying information
#[openapi(tag = "Authentication")]
#[post("/auth/minecraft", data = "<payload>")]
pub async fn authenticate(
    db: Db<'_>,
    payload: Json<LoginRequest>,
    config: &State<Server>,
    identity_service: &State<PlayerIdentityService>,
    perm_config: &State<Permissions>,
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

    // Create the Minecraft auth provider
    let provider = MinecraftAuthProvider::new(config.minecraft.client_id.clone());

    // Authenticate with Xbox Live
    let auth_result = match provider.authenticate(code, redirect_uri).await {
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

    // Build login response using AuthService
    let perm_service = PermissionService::new(perm_config.defaults.clone());
    match AuthService::build_login_response(
        conn,
        config.inner(),
        Some(&perm_service),
        gamertag.clone(),
        auth_result.gamerpic,
        Game::Minecraft,
    )
    .await
    {
        Ok(mut response) => {
            // Store the MC Java username alias if it differs from the gamertag
            if let Some(ref mc_name) = minecraft_username {
                if mc_name != &gamertag {
                    if let Some(player_id) = identity_service
                        .find_player_id_by_gamertag(&gamertag, &Game::Minecraft)
                        .await
                    {
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
                                "Failed to create identity alias for {}: {}",
                                mc_name,
                                e
                            );
                        }
                    }
                }
            }

            // Include MC username in response for client display
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
