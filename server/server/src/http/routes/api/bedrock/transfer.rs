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
/// Used by Proxy Connect and Realms Connect to hand a client session off to the real
/// backend server.
#[openapi(tag = "Bedrock")]
#[post("/bedrock/transfer", data = "<body>")]
pub async fn register_transfer_target(
    cert: rocket::mtls::Certificate<'_>,
    body: Json<TransferTargetRequest>,
    cache: &State<TransferTargetCache>,
    server_config: &State<Server>,
    db: Db<'_>,
) -> Result<Json<TransferTargetResponse>, rocket::http::Status> {
    let conn = db.into_inner();
    let _player = AuthService::player_from_certificate(&cert, conn, Some("minecraft")).await?;

    cache.set(&body.xuid, body.host.clone(), body.port).await;

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
