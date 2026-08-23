use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::structs::audio::LevelSnapshot;

/// Everyone's voice activity, as one push frame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct LevelsPush {
    #[serde(rename = "type")]
    pub kind: String,
    pub data: LevelSnapshot,
}

impl LevelsPush {
    pub const KIND: &'static str = "levels";

    pub fn new(data: LevelSnapshot) -> Self {
        Self {
            kind: Self::KIND.to_string(),
            data,
        }
    }
}
