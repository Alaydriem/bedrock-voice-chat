use std::sync::Arc;

use rocket::{http::Status, mtls::Certificate, serde::json::Json, State};
use crate::http::openapi::CustomJsonResponse;
use rocket_okapi::openapi;
use common::request::AudioPlayRequest;
use common::response::AudioEventResponse;

use crate::http::guards::MCAccessToken;
use crate::http::pool::Db;
use crate::services::AudioPlaybackService;

#[openapi(tag = "Audio")]
#[post("/event", data = "<request>")]
pub async fn audio_event_play(
    db: Db<'_>,
    _token: MCAccessToken,
    playback_service: &State<Arc<AudioPlaybackService>>,
    request: Json<AudioPlayRequest>,
) -> CustomJsonResponse<AudioEventResponse> {
    let conn = db.into_inner();

    match playback_service
        .start_playback(conn, request.into_inner())
        .await
    {
        Ok(response) => CustomJsonResponse::ok(response),
        Err(e) => {
            tracing::error!("Failed to start playback: {}", e);
            CustomJsonResponse::error(Status::InternalServerError)
        }
    }
}

#[openapi(tag = "Audio")]
#[delete("/event/<event_id>")]
pub async fn audio_event_stop(
    _token: MCAccessToken,
    playback_service: &State<Arc<AudioPlaybackService>>,
    event_id: &str,
) -> Status {
    match playback_service.stop_playback(event_id).await {
        Ok(_) => Status::Ok,
        Err(_) => Status::NotFound,
    }
}
