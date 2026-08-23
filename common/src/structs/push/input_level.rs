use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::structs::audio::InputLevel;

/// The unquantised capture level, for the screens that calibrate a microphone.
///
/// Separate from `LevelsPush` because it carries a different measurement for a different
/// purpose: the meter scale is quantised to hold the message count down, and calibration needs
/// the amplitude that quantisation discards.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct InputLevelPush {
    #[serde(rename = "type")]
    pub kind: String,
    pub data: InputLevel,
}

impl InputLevelPush {
    pub const KIND: &'static str = "input_level";

    pub fn new(data: InputLevel) -> Self {
        Self {
            kind: Self::KIND.to_string(),
            data,
        }
    }
}
