use common::{
    auth::{AuthError as CommonAuthError, MinecraftAuthProvider},
    ncryptflib as ncryptf,
    structs::config::{LoginRequest, LoginResponse},
    Game,
};
use rocket::{http::Status, serde::json::Json, State};

use sea_orm_rocket::Connection as SeaOrmConnection;

use crate::config::ApplicationConfigServer;
use crate::rs::pool::AppDb;
use crate::rs::structs::ncryptf_json::JsonMessage;
use crate::rs::structs::{build_login_response, AuthError};

/// Authenticates the Player via Xbox Live to grab their gamertag and other identifying information
#[post("/auth/minecraft", data = "<payload>")]
pub async fn authenticate(
    db: SeaOrmConnection<'_, AppDb>,
    payload: Json<LoginRequest>,
    config: &State<ApplicationConfigServer>,
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

    // Build login response using shared logic
    match build_login_response(
        conn,
        config.inner(),
        auth_result.gamertag,
        auth_result.gamerpic,
        Game::Minecraft,
    )
    .await
    {
        Ok(response) => JsonMessage::create(Status::Ok, Some(response), None, None),
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
