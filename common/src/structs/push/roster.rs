use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::structs::voice::VoiceRoster;

/// Who this client can hear, as one push frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct RosterPush {
    #[serde(rename = "type")]
    pub kind: String,
    pub data: VoiceRoster,
}

impl RosterPush {
    pub const KIND: &'static str = "roster";

    pub fn new(data: VoiceRoster) -> Self {
        Self {
            kind: Self::KIND.to_string(),
            data,
        }
    }
}
