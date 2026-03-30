use entity::{audio_file, player};
use common::response::{AudioFileResponse, PaginatedResponse};
use common::structs::game::UploaderIdentity;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, Order,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, RelationTrait,
};
use sea_orm::sea_query::JoinType;

use crate::config::Audio;
use crate::services::audio_playback_service::ogg_opus_parser::OggOpusParser;
use crate::services::AudioPlaybackService;

pub struct AudioFileService;

impl AudioFileService {
    pub async fn upload<C: ConnectionTrait>(
        conn: &C,
        player_id: i32,
        gamertag: String,
        game: String,
        bytes: Vec<u8>,
        filename: Option<String>,
        config: &Audio,
    ) -> Result<AudioFileResponse, AudioFileError> {
        if bytes.len() < 4 || &bytes[0..4] != b"OggS" {
            return Err(AudioFileError::InvalidFormat);
        }

        let bytes_clone = bytes.clone();
        let (duration_ms, _frame_count) = tokio::task::spawn_blocking(move || {
            OggOpusParser::parse_duration(&bytes_clone)
        })
        .await
        .map_err(|_| AudioFileError::ParseFailed)?
        .map_err(|_| AudioFileError::ParseFailed)?;

        if duration_ms > 600_000 {
            return Err(AudioFileError::AudioTooLong);
        }

        let file_id = uuid::Uuid::now_v7().to_string();
        let audio_dir = config.file_path.clone();

        tokio::fs::create_dir_all(&audio_dir)
            .await
            .map_err(|e| {
                tracing::error!("Failed to create audio directory: {}", e);
                AudioFileError::Internal
            })?;

        let file_path = format!("{}/{}.opus", audio_dir, file_id);
        tokio::fs::write(&file_path, &bytes).await.map_err(|e| {
            tracing::error!("Failed to write audio file: {}", e);
            AudioFileError::Internal
        })?;

        let now = common::ncryptflib::rocket::Utc::now().timestamp();

        let active_model = audio_file::ActiveModel {
            id: ActiveValue::Set(file_id.clone()),
            uploader_id: ActiveValue::Set(player_id),
            original_filename: ActiveValue::Set(
                filename.unwrap_or_else(|| "uploaded.opus".to_string()),
            ),
            duration_ms: ActiveValue::Set(duration_ms as i64),
            file_size_bytes: ActiveValue::Set(bytes.len() as i64),
            game: ActiveValue::Set(game.clone()),
            deleted: ActiveValue::Set(0),
            created_at: ActiveValue::Set(now),
        };

        match active_model.insert(conn).await {
            Ok(model) => Ok(Self::to_response(model, gamertag)),
            Err(e) => {
                tracing::error!("Failed to insert audio file record: {}", e);
                let _ = tokio::fs::remove_file(&file_path).await;
                Err(AudioFileError::Internal)
            }
        }
    }

    pub async fn list<C: ConnectionTrait>(
        conn: &C,
        page: u32,
        page_size: u32,
        sort_by: Option<String>,
        sort_order: Option<String>,
        search: Option<String>,
    ) -> Result<PaginatedResponse<AudioFileResponse>, AudioFileError> {
        let page_size = page_size.min(100);

        let mut base = audio_file::Entity::find()
            .filter(audio_file::Column::Deleted.eq(0));

        if let Some(ref search) = search {
            if !search.is_empty() {
                base = base.filter(audio_file::Column::OriginalFilename.contains(search));
            }
        }

        let sort_column = match sort_by.as_deref() {
            Some("original_filename") => audio_file::Column::OriginalFilename,
            Some("duration_ms") => audio_file::Column::DurationMs,
            Some("file_size_bytes") => audio_file::Column::FileSizeBytes,
            _ => audio_file::Column::CreatedAt,
        };

        let order = match sort_order.as_deref() {
            Some("asc") => Order::Asc,
            _ => Order::Desc,
        };

        base = base.order_by(sort_column, order);

        let total = base.clone().count(conn).await.map_err(|e| {
            tracing::error!("Failed to count audio files: {}", e);
            AudioFileError::Internal
        })? as u32;

        let offset = (page as u64) * (page_size as u64);
        let results = base
            .find_also_related(player::Entity)
            .offset(Some(offset))
            .limit(Some(page_size as u64))
            .all(conn)
            .await
            .map_err(|e| {
                tracing::error!("Failed to list audio files: {}", e);
                AudioFileError::Internal
            })?;

        let items = results
            .into_iter()
            .map(|(file, player)| {
                let gamertag = player
                    .and_then(|p| p.gamertag)
                    .unwrap_or_default();
                Self::to_response(file, gamertag)
            })
            .collect();

        Ok(PaginatedResponse {
            items,
            total,
            page,
            page_size,
        })
    }

    pub async fn delete<C: ConnectionTrait>(
        conn: &C,
        player_id: i32,
        file_id: &str,
        playback_service: &AudioPlaybackService,
        config: &Audio,
    ) -> Result<(), AudioFileError> {
        let file = audio_file::Entity::find_by_id(file_id.to_string())
            .filter(audio_file::Column::Deleted.eq(0))
            .one(conn)
            .await
            .map_err(|_| AudioFileError::Internal)?
            .ok_or(AudioFileError::NotFound)?;

        if file.uploader_id != player_id {
            return Err(AudioFileError::Forbidden);
        }

        if playback_service.is_file_playing(&file.id).await {
            let mut active: audio_file::ActiveModel = file.into();
            active.deleted = ActiveValue::Set(1);
            active
                .update(conn)
                .await
                .map_err(|_| AudioFileError::Internal)?;
        } else {
            let file_path = format!("{}/{}.opus", config.file_path, file.id);
            let id = file.id.clone();
            audio_file::Entity::delete_by_id(id)
                .exec(conn)
                .await
                .map_err(|_| AudioFileError::Internal)?;
            let _ = tokio::fs::remove_file(&file_path).await;
        }

        Ok(())
    }

    pub async fn cleanup_deleted<C: ConnectionTrait>(
        conn: &C,
        storage_path: &str,
    ) -> Result<usize, AudioFileError> {
        let deleted_files = audio_file::Entity::find()
            .filter(audio_file::Column::Deleted.eq(1))
            .all(conn)
            .await
            .map_err(|e| {
                tracing::error!("Failed to query deleted audio files: {}", e);
                AudioFileError::Internal
            })?;

        let count = deleted_files.len();
        for file in deleted_files {
            let file_path = format!("{}/{}.opus", storage_path, file.id);
            let id = file.id.clone();
            let _ = audio_file::Entity::delete_by_id(id).exec(conn).await;
            let _ = tokio::fs::remove_file(&file_path).await;
        }

        if count > 0 {
            tracing::info!("Cleaned up {} soft-deleted audio files", count);
        }
        Ok(count)
    }

    pub async fn exists<C: ConnectionTrait>(
        conn: &C,
        file_id: &str,
    ) -> Result<bool, AudioFileError> {
        let count = audio_file::Entity::find_by_id(file_id.to_string())
            .filter(audio_file::Column::Deleted.eq(0))
            .count(conn)
            .await
            .map_err(|e| {
                tracing::error!("Failed to check audio file existence: {}", e);
                AudioFileError::Internal
            })?;
        Ok(count > 0)
    }

    fn to_response(model: audio_file::Model, gamertag: String) -> AudioFileResponse {
        let uploader = UploaderIdentity::from_game_str(&model.game, gamertag);
        AudioFileResponse {
            id: model.id,
            uploader,
            original_filename: model.original_filename,
            duration_ms: model.duration_ms as i32,
            file_size_bytes: model.file_size_bytes as i32,
            game: model.game,
            created_at: model.created_at as i32,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AudioFileError {
    #[error("invalid audio format")]
    InvalidFormat,
    #[error("failed to parse audio file")]
    ParseFailed,
    #[error("audio exceeds maximum duration")]
    AudioTooLong,
    #[error("audio file not found")]
    NotFound,
    #[error("not authorized")]
    Forbidden,
    #[error("internal server error")]
    Internal,
}

impl AudioFileError {
    pub fn status(&self) -> rocket::http::Status {
        match self {
            AudioFileError::InvalidFormat => rocket::http::Status::UnsupportedMediaType,
            AudioFileError::ParseFailed => rocket::http::Status::UnprocessableEntity,
            AudioFileError::AudioTooLong => rocket::http::Status::UnprocessableEntity,
            AudioFileError::NotFound => rocket::http::Status::NotFound,
            AudioFileError::Forbidden => rocket::http::Status::Forbidden,
            AudioFileError::Internal => rocket::http::Status::InternalServerError,
        }
    }
}
