pub mod manifest;

pub use manifest::SessionManifest;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct RecordingSession {
    pub session_data: SessionManifest,
    pub file_size_mb: f64,
    pub recording_path: String,
    pub exportable: bool,
}
