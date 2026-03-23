use std::sync::Arc;

use rocket::{
    data::{Data, ToByteUnit},
    http::Status,
    mtls::Certificate,
    State,
};
use crate::http::openapi::CustomJsonResponse;
use rocket_okapi::openapi;

use common::response::{AudioFileResponse, PaginatedResponse};
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
#[get("/file?<page>&<page_size>&<sort_by>&<sort_order>&<search>")]
pub async fn audio_file_list(
    identity: Certificate<'_>,
    db: Db<'_>,
    page: Option<u32>,
    page_size: Option<u32>,
    sort_by: Option<String>,
    sort_order: Option<String>,
    search: Option<String>,
) -> CustomJsonResponse<PaginatedResponse<AudioFileResponse>> {
    let conn = db.into_inner();

    if let Err(status) = AuthService::player_from_certificate(&identity, conn, None).await {
        return CustomJsonResponse::error(status);
    }

    let page = page.unwrap_or(0);
    let page_size = page_size.unwrap_or(20);

    match AudioFileService::list(conn, page, page_size, sort_by, sort_order, search).await {
        Ok(result) => CustomJsonResponse::ok(result),
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
