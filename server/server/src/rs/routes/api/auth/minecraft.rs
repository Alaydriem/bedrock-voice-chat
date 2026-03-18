use common::{
    auth::{AuthError as CommonAuthError, MinecraftAuthProvider},
    ncryptflib as ncryptf,
    structs::config::{LoginRequest, LoginResponse},
    Game,
};
use rocket::{http::Status, serde::json::Json, State};

use sea_orm_rocket::Connection as SeaOrmConnection;

use crate::config::Server;
use crate::rs::pool::AppDb;
use crate::rs::dtos::ncryptf::JsonMessage;
use crate::services::{AuthError, AuthService, PlayerIdentityService};

/// Authenticates the Player via Xbox Live to grab their gamertag and other identifying information
#[post("/auth/minecraft", data = "<payload>")]
pub async fn authenticate(
    db: SeaOrmConnection<'_, AppDb>,
    payload: Json<LoginRequest>,
    config: &State<Server>,
    identity_service: &State<PlayerIdentityService>
) -> ncryptf::rocket::JsonResponse<JsonMessage<LoginResponse>> {
    let conn = db.into_inner();

    let code = payload.0.code;
    let redirect_uri = match payload.0.redirect_uri.parse() {
        Ok(uri) => uri,
        Err(e) => {
            tracing::error!("Invalid redirect URI: {}", e);
            return JsonMessage::create(Status::BadRequest, None, None, None);
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
                    JsonMessage::create(Status::Forbidden, None, None, None)
                }
                _ => JsonMessage::create(Status::Forbidden, None, None, None),
            };
        }
    };

    let gamertag = auth_result.gamertag.clone();
    let minecraft_username = auth_result.minecraft_username.clone();

    // Build login response using AuthService
    match AuthService::build_login_response(
        conn,
        config.inner(),
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

            JsonMessage::create(Status::Ok, Some(response), None, None)
        }
        Err(e) => {
            tracing::error!("Login failed: {}", e);
            match e {
                AuthError::PlayerNotFound | AuthError::PlayerBanished => {
                    JsonMessage::create(Status::Forbidden, None, None, None)
                }
                _ => JsonMessage::create(Status::InternalServerError, None, None, None),
            }
        }
    }
}
