use std::sync::Arc;

use entity::audio_file;
use rocket::{
    data::{Data, ToByteUnit},
    serde::json::Json,
    State,
};
use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter};
use sea_orm_rocket::Connection as SeaOrmConnection;

use common::response::AudioFileResponse;
use common::response::error::ApiError;

use crate::rs::pool::AppDb;
use crate::services::audio_playback_service::ogg_opus_parser::OggOpusParser;
use crate::services::{AudioPlaybackService, AuthService};
use super::{GameHint, MtlsIdentity, RocketApiError};
use super::original_filename::OriginalFilename;

/// Upload an audio file (Ogg/Opus).
/// Auth: mTLS client certificate
#[post("/audio/file", data = "<data>")]
pub async fn audio_file_upload(
    identity: MtlsIdentity<'_>,
    original_filename: OriginalFilename,
    game_hint: GameHint,
    db: SeaOrmConnection<'_, AppDb>,
    config: &State<crate::config::ApplicationConfigServer>,
    data: Data<'_>,
) -> Result<Json<AudioFileResponse>, RocketApiError> {
    let conn = db.into_inner();

    let player = AuthService::player_from_certificate(&identity.0, conn, game_hint.0.as_deref())
        .await
        .map_err(|_| RocketApiError::from(ApiError::AuthFailed))?;

    let bytes = match data.open(10.megabytes()).into_bytes().await {
        Ok(b) if b.is_complete() => b.into_inner(),
        _ => return Err(ApiError::FileTooLarge { max_bytes: 10 * 1024 * 1024 }.into()),
    };

    tracing::debug!(
        byte_count = bytes.len(),
        first_bytes = ?&bytes[..std::cmp::min(16, bytes.len())],
        "Received upload data"
    );

    if bytes.len() < 4 || &bytes[0..4] != b"OggS" {
        return Err(ApiError::InvalidFormat.into());
    }

    let bytes_clone = bytes.clone();
    let parse_result = tokio::task::spawn_blocking(move || {
        OggOpusParser::parse_duration(&bytes_clone)
    })
    .await
    .map_err(|_| RocketApiError::from(ApiError::Internal))?
    .map_err(|e| {
        tracing::error!("Ogg/Opus parse error: {}", e);
        RocketApiError::from(ApiError::ParseFailed)
    })?;

    let (duration_ms, _frame_count) = parse_result;

    if duration_ms > 600_000 {
        return Err(ApiError::AudioTooLong { max_duration_ms: 600_000 }.into());
    }

    let file_id = uuid::Uuid::now_v7().to_string();
    let audio_path = &config.inner().assets_path;
    let audio_dir = format!("{}/audio", audio_path);

    if let Err(e) = std::fs::create_dir_all(&audio_dir) {
        tracing::error!("Failed to create audio directory: {}", e);
        return Err(ApiError::Internal.into());
    }

    let file_path = format!("{}/{}.opus", audio_dir, file_id);
    if let Err(e) = std::fs::write(&file_path, &bytes) {
        tracing::error!("Failed to write audio file: {}", e);
        return Err(ApiError::Internal.into());
    }

    let now = common::ncryptflib::rocket::Utc::now().timestamp();

    let filename = original_filename
        .0
        .unwrap_or_else(|| "uploaded.opus".to_string());

    let active_model = audio_file::ActiveModel {
        id: ActiveValue::Set(file_id.clone()),
        uploader_id: ActiveValue::Set(player.id),
        original_filename: ActiveValue::Set(filename),
        duration_ms: ActiveValue::Set(duration_ms as i64),
        file_size_bytes: ActiveValue::Set(bytes.len() as i64),
        game: ActiveValue::Set(player.game.to_string()),
        deleted: ActiveValue::Set(0),
        created_at: ActiveValue::Set(now),
    };

    match active_model.insert(conn).await {
        Ok(model) => Ok(Json(AudioFileResponse {
            id: model.id,
            uploader_id: model.uploader_id,
            original_filename: model.original_filename,
            duration_ms: model.duration_ms,
            file_size_bytes: model.file_size_bytes,
            game: model.game,
            created_at: model.created_at,
        })),
        Err(e) => {
            tracing::error!("Failed to insert audio file record: {}", e);
            let _ = std::fs::remove_file(&file_path);
            Err(ApiError::Internal.into())
        }
    }
}

/// List all non-deleted audio files.
/// Auth: mTLS client certificate
#[get("/audio/file")]
pub async fn audio_file_list(
    _identity: MtlsIdentity<'_>,
    _game_hint: GameHint,
    db: SeaOrmConnection<'_, AppDb>,
) -> Result<Json<Vec<AudioFileResponse>>, RocketApiError> {
    let conn = db.into_inner();

    match audio_file::Entity::find()
        .filter(audio_file::Column::Deleted.eq(0))
        .all(conn)
        .await
    {
        Ok(files) => Ok(Json(
            files
                .into_iter()
                .map(|model| AudioFileResponse {
                    id: model.id,
                    uploader_id: model.uploader_id,
                    original_filename: model.original_filename,
                    duration_ms: model.duration_ms,
                    file_size_bytes: model.file_size_bytes,
                    game: model.game,
                    created_at: model.created_at,
                })
                .collect(),
        )),
        Err(e) => {
            tracing::error!("Failed to list audio files: {}", e);
            Err(ApiError::Internal.into())
        }
    }
}

/// Delete an audio file (soft-delete if playing, hard-delete otherwise).
/// Auth: mTLS + owner check
#[delete("/audio/file/<file_id>")]
pub async fn audio_file_delete(
    identity: MtlsIdentity<'_>,
    game_hint: GameHint,
    db: SeaOrmConnection<'_, AppDb>,
    playback_service: &State<Arc<AudioPlaybackService>>,
    config: &State<crate::config::ApplicationConfigServer>,
    file_id: &str,
) -> Result<Json<serde_json::Value>, RocketApiError> {
    let conn = db.into_inner();

    let player = AuthService::player_from_certificate(&identity.0, conn, game_hint.0.as_deref())
        .await
        .map_err(|_| RocketApiError::from(ApiError::AuthFailed))?;

    let file = audio_file::Entity::find_by_id(file_id.to_string())
        .filter(audio_file::Column::Deleted.eq(0))
        .one(conn)
        .await
        .map_err(|_| RocketApiError::from(ApiError::Internal))?
        .ok_or_else(|| RocketApiError::from(ApiError::NotFound))?;

    if file.uploader_id != player.id {
        return Err(ApiError::Forbidden.into());
    }

    if playback_service.is_file_playing(&file.id).await {
        let mut active: audio_file::ActiveModel = file.into();
        active.deleted = ActiveValue::Set(1);
        active
            .update(conn)
            .await
            .map_err(|_| RocketApiError::from(ApiError::Internal))?;
        Ok(Json(serde_json::json!({ "deleted": true, "soft": true })))
    } else {
        let file_path = format!(
            "{}/audio/{}.opus",
            config.inner().assets_path, file.id
        );
        let file_model = file;
        audio_file::Entity::delete_by_id(file_model.id.clone())
            .exec(conn)
            .await
            .map_err(|_| RocketApiError::from(ApiError::Internal))?;
        let _ = std::fs::remove_file(&file_path);
        Ok(Json(serde_json::json!({ "deleted": true, "soft": false })))
    }
}
