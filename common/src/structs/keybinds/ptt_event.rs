use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum PttEvent {
    #[serde(rename = "ptt:active")]
    Active,
}

impl std::fmt::Display for PttEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PttEvent::Active => write!(f, "ptt:active"),
        }
    }
}
