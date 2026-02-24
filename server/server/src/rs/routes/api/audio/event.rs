use std::sync::Arc;

use rocket::serde::json::Json;
use rocket::State;
use sea_orm_rocket::Connection as SeaOrmConnection;

use common::request::audio::AudioPlayRequest;
use common::response::AudioEventResponse;
use common::response::error::ApiError;

use crate::rs::guards::MCAccessToken;
use crate::rs::pool::AppDb;
use crate::services::AudioPlaybackService;
use super::RocketApiError;

/// Start audio playback at a location.
/// Auth: MCAccessToken (from game plugin)
#[post("/audio/event", data = "<request>")]
pub async fn audio_event_play(
    db: SeaOrmConnection<'_, AppDb>,
    _token: MCAccessToken,
    playback_service: &State<Arc<AudioPlaybackService>>,
    request: Json<AudioPlayRequest>,
) -> Result<Json<AudioEventResponse>, RocketApiError> {
    let conn = db.into_inner();

    playback_service
        .start_playback(conn, request.into_inner())
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to start playback: {}", e);
            if e.contains("not found") {
                RocketApiError::from(ApiError::NotFound)
            } else if e.contains("Duplicate") {
                RocketApiError::from(ApiError::Duplicate)
            } else {
                RocketApiError::from(ApiError::Internal)
            }
        })
}

/// Stop an active audio playback session.
/// Auth: MCAccessToken (from game plugin)
#[delete("/audio/event/<event_id>")]
pub async fn audio_event_stop(
    _token: MCAccessToken,
    playback_service: &State<Arc<AudioPlaybackService>>,
    event_id: &str,
) -> Result<Json<serde_json::Value>, RocketApiError> {
    playback_service
        .stop_playback(event_id)
        .await
        .map(|_| Json(serde_json::json!({ "success": true })))
        .map_err(|e| {
            tracing::error!("Failed to stop playback: {}", e);
            RocketApiError::from(ApiError::NotFound)
        })
}
