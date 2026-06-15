use std::sync::Arc;

use entity::audio_file;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter};

// Whether this server holds a given audio file locally. Abstracted so the
// responder can be exercised without a database or real files on disk.
#[async_trait::async_trait]
pub trait AudioFileExistence: Send + Sync {
    async fn has_audio(&self, audio_id: &str) -> bool;
}

// Production check: the file must have a non-deleted `audio_file` row AND its
// `{audio_storage_path}/{id}.opus` must exist on disk — the same pair the
// playback path resolves before streaming.
pub struct DbAudioFileExistence {
    db: Arc<DatabaseConnection>,
    audio_storage_path: String,
}

impl DbAudioFileExistence {
    pub fn new(db: Arc<DatabaseConnection>, audio_storage_path: String) -> Self {
        Self {
            db,
            audio_storage_path,
        }
    }

    pub fn new_shared(db: Arc<DatabaseConnection>, audio_storage_path: String) -> Arc<Self> {
        Arc::new(Self::new(db, audio_storage_path))
    }
}

#[async_trait::async_trait]
impl AudioFileExistence for DbAudioFileExistence {
    async fn has_audio(&self, audio_id: &str) -> bool {
        let row_exists = audio_file::Entity::find_by_id(audio_id.to_string())
            .filter(audio_file::Column::Deleted.eq(0))
            .count(self.db.as_ref())
            .await
            .map(|count| count > 0)
            .unwrap_or(false);
        if !row_exists {
            return false;
        }
        let file_path = format!("{}/{}.opus", self.audio_storage_path, audio_id);
        tokio::fs::try_exists(&file_path).await.unwrap_or(false)
    }
}
