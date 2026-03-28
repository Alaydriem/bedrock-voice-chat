use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::structs::game::UploaderIdentity;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct AudioFileResponse {
    pub id: String,
    pub uploader: UploaderIdentity,
    pub original_filename: String,
    pub duration_ms: i32,
    pub file_size_bytes: i32,
    pub game: String,
    pub created_at: i32,
}
