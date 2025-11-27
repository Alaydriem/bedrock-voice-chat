use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct NoiseGateSettings {
    pub open_threshold: f32,
    pub close_threshold: f32,
    pub release_rate: f32,
    pub attack_rate: f32,
    pub hold_time: f32,
}

impl Default for NoiseGateSettings {
    fn default() -> Self {
        Self {
            open_threshold: -36.0,
            close_threshold: -56.0,
            release_rate: 150.0,
            attack_rate: 5.0,
            hold_time: 150.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct PlayerGainSettings {
    pub gain: f32,
    pub muted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct PlayerGainStore(pub HashMap<String, PlayerGainSettings>);

impl Default for PlayerGainStore {
    fn default() -> Self {
        Self(std::collections::HashMap::new())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum StreamEvent {
    Mute,
    Record
}

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