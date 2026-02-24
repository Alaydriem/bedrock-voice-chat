use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Audio output format selection for rendering recordings
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum AudioFormat {
    /// Broadcast WAV with BEXT metadata (uncompressed PCM)
    Bwav,
    /// MP4/M4A with Opus audio (compressed, lossless passthrough)
    Mp4Opus,
}

impl AudioFormat {
    /// Returns the file extension for this format (without dot)
    pub fn extension(&self) -> &'static str {
        match self {
            AudioFormat::Bwav => "wav",
            AudioFormat::Mp4Opus => "m4a",
        }
    }
}
