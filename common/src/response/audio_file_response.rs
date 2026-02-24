use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Audio file metadata returned from upload, list, and get endpoints.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct AudioFileResponse {
    pub id: String,
    pub uploader_id: i32,
    pub original_filename: String,
    pub duration_ms: i64,
    pub file_size_bytes: i64,
    pub game: String,
    pub created_at: i64,
}
