use std::sync::Arc;

use crate::http::openapi::CustomJsonResponse;
use common::request::{AudioPlayRequest, GameAudioContext};
use common::response::AudioEventResponse;
use rocket::{State, http::Status, serde::json::Json};
use rocket_okapi::openapi;

use crate::http::guards::GameAccessToken;
use crate::http::pool::Db;
use crate::services::{AudioPlaybackService, BedrockEventService};

/// Start playback of an audio clip as a positioned in-world event.
#[openapi(tag = "Audio")]
#[post("/event", data = "<request>")]
pub async fn audio_event_play(
    db: Db<'_>,
    _token: GameAccessToken,
    playback_service: &State<Arc<AudioPlaybackService>>,
    bedrock_event_service: &State<Arc<BedrockEventService>>,
    request: Json<AudioPlayRequest>,
) -> CustomJsonResponse<AudioEventResponse> {
    let conn = db.into_inner();
    let request = request.into_inner();

    if let GameAudioContext::Minecraft(ctx) = &request.game {
        if !ctx.world_uuid.is_empty() {
            bedrock_event_service
                .notify_addon_http(&ctx.world_uuid)
                .await;
        }
    }

    match playback_service.start_playback(conn, request).await {
        Ok(response) => CustomJsonResponse::ok(response),
        Err(e) => {
            tracing::error!("Failed to start playback: {}", e);
            CustomJsonResponse::error(Status::InternalServerError)
        }
    }
}

/// Stop a playing audio event.
#[openapi(tag = "Audio")]
#[delete("/event/<event_id>")]
pub async fn audio_event_stop(
    _token: GameAccessToken,
    playback_service: &State<Arc<AudioPlaybackService>>,
    event_id: &str,
) -> Status {
    match playback_service.stop_playback(event_id).await {
        Ok(_) => Status::Ok,
        Err(_) => Status::NotFound,
    }
}
