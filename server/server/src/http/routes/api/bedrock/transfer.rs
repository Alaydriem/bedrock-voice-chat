use crate::http::guards::PlayerGuard;
use common::request::bedrock::TransferTargetRequest;
use common::response::bedrock::TransferTargetResponse;
use rocket::State;
use rocket::serde::json::Json;
use rocket_okapi::openapi;

use crate::config::Server;
use crate::http::pool::Db;
use crate::services::auth_service::AuthService;
use crate::services::bedrock::TransferTargetCache;

/// Register a Bedrock transfer target for the relay.
///
/// Used by BVC Connect to hand a client session off to the real backend server.
///
/// The target is stored under the caller's own gamertag. The request carries no subject, so
/// a player cannot point somebody else's handoff at a server of their choosing.
#[openapi(tag = "Bedrock")]
#[post("/bedrock/transfer", data = "<body>")]
pub async fn register_transfer_target(
    guard: PlayerGuard,
    body: Json<TransferTargetRequest>,
    cache: &State<TransferTargetCache>,
    server_config: &State<Server>,
    db: Db<'_>,
) -> Result<Json<TransferTargetResponse>, rocket::http::Status> {
    let conn = db.into_inner();
    let player = guard.player;

    // Asserted rather than assumed. This route is Bedrock-only, and the previous code told
    // the auth service the certificate was a Minecraft one instead of checking that it was.
    if player.game != common::Game::Minecraft {
        return Err(rocket::http::Status::Forbidden);
    }

    let gamertag = player.gamertag.clone().ok_or(rocket::http::Status::Forbidden)?;
    cache.set(&gamertag, body.host.clone(), body.port).await;

    let ttl_secs = server_config.bedrock.transfer_cache_ttl_secs;
    let expires_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        + ttl_secs;

    Ok(Json(TransferTargetResponse {
        host: body.host.clone(),
        port: body.port,
        expires_at,
    }))
}
