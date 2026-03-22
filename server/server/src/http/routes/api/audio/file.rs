use std::sync::Arc;

use rocket::{
    data::{Data, ToByteUnit},
    http::Status,
    mtls::Certificate,
    State,
};
use crate::http::openapi::CustomJsonResponse;
use rocket_okapi::openapi;

use common::response::AudioFileResponse;
use common::structs::permission::Permission;

use crate::config::{Audio, Permissions};
use crate::http::guards::OriginalFilename;
use crate::http::pool::Db;
use crate::services::{AuthService, AudioFileService, AudioPlaybackService, PermissionService};

#[openapi(skip)]
#[post("/file", data = "<data>")]
pub async fn audio_file_upload(
    identity: Certificate<'_>,
    filename: Option<OriginalFilename>,
    db: Db<'_>,
    config: &State<Audio>,
    perm_config: &State<Permissions>,
    data: Data<'_>,
) -> CustomJsonResponse<AudioFileResponse> {
    let conn = db.into_inner();

    let player = match AuthService::player_from_certificate(&identity, conn, None).await {
        Ok(p) => p,
        Err(status) => return CustomJsonResponse::error(status),
    };

    let perm_service = PermissionService::new(perm_config.defaults.clone());
    if !perm_service
        .evaluate(conn, player.id, &Permission::AudioUpload)
        .await
    {
        return CustomJsonResponse::error(Status::Forbidden);
    }

    let bytes = match data.open(10.megabytes()).into_bytes().await {
        Ok(b) if b.is_complete() => b.into_inner(),
        _ => return CustomJsonResponse::error(Status::PayloadTooLarge),
    };

    let gamertag = player.gamertag.clone().unwrap_or_default();
    match AudioFileService::upload(
        conn,
        player.id,
        gamertag,
        player.game.to_string(),
        bytes,
        filename.map(|f| f.0),
        config.inner(),
    )
    .await
    {
        Ok(response) => CustomJsonResponse::custom(Status::Created, Some(response)),
        Err(e) => CustomJsonResponse::error(e.status()),
    }
}

#[openapi(tag = "Audio")]
#[get("/file")]
pub async fn audio_file_list(
    identity: Certificate<'_>,
    db: Db<'_>,
) -> CustomJsonResponse<Vec<AudioFileResponse>> {
    let conn = db.into_inner();

    if let Err(status) = AuthService::player_from_certificate(&identity, conn, None).await {
        return CustomJsonResponse::error(status);
    }

    match AudioFileService::list(conn).await {
        Ok(files) => CustomJsonResponse::ok(files),
        Err(e) => CustomJsonResponse::error(e.status()),
    }
}

#[openapi(tag = "Audio")]
#[delete("/file/<file_id>")]
pub async fn audio_file_delete(
    identity: Certificate<'_>,
    db: Db<'_>,
    playback_service: &State<Arc<AudioPlaybackService>>,
    config: &State<Audio>,
    perm_config: &State<Permissions>,
    file_id: &str,
) -> Status {
    let conn = db.into_inner();

    let player = match AuthService::player_from_certificate(&identity, conn, None).await {
        Ok(p) => p,
        Err(status) => return status,
    };

    let perm_service = PermissionService::new(perm_config.defaults.clone());
    if !perm_service
        .evaluate(conn, player.id, &Permission::AudioDelete)
        .await
    {
        return Status::Forbidden;
    }

    match AudioFileService::delete(conn, player.id, file_id, &playback_service, config.inner())
        .await
    {
        Ok(()) => Status::Ok,
        Err(e) => e.status(),
    }
}
