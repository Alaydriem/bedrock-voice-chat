use common::{
    auth::MinecraftAuthProvider,
    request::LinkJavaIdentityRequest,
    response::LinkJavaIdentityResponse,
    Game,
};
use rocket::{http::Status, mtls::Certificate, serde::json::Json, State};

use crate::services::PlayerIdentityService;

/// Links a Java Minecraft identity to an existing player.
/// Requires mTLS client certificate for authentication.
/// The client sends an OAuth code which is exchanged for MC Services credentials
/// to discover the player's Java username.
#[post("/auth/link-java", data = "<payload>")]
pub async fn link_java_identity<'r>(
    _identity: Certificate<'r>,
    payload: Json<LinkJavaIdentityRequest>,
    identity_service: &State<PlayerIdentityService>,
) -> Result<Json<LinkJavaIdentityResponse>, Status> {
    let request = payload.0;

    let redirect_uri = request
        .redirect_uri
        .parse()
        .map_err(|_| Status::BadRequest)?;

    let provider = MinecraftAuthProvider::new(request.client_id);

    // Only fetch the MC Java profile — skip full Xbox profile lookup
    let mc_username = provider
        .authenticate_for_java_profile(request.code, redirect_uri)
        .await
        .map_err(|e| {
            tracing::error!("Link Java identity auth failed: {}", e);
            Status::Forbidden
        })?;

    // Store the alias if the MC username differs from the gamertag
    if mc_username != request.gamertag {
        if let Some(player_id) = identity_service
            .find_player_id_by_gamertag(&request.gamertag, &Game::Minecraft)
            .await
        {
            if let Err(e) = identity_service
                .create_alias(player_id, &mc_username, &Game::Minecraft, "minecraft_services")
                .await
            {
                tracing::warn!("Failed to create identity alias for {}: {}", mc_username, e);
            }
        }
    }

    Ok(Json(LinkJavaIdentityResponse {
        minecraft_username: Some(mc_username),
    }))
}
