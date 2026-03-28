use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum AudioFormat {
    Bwav,
    Mp4Opus,
}

impl AudioFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            AudioFormat::Bwav => "wav",
            AudioFormat::Mp4Opus => "m4a",
        }
    }
}
