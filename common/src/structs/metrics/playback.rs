use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct PlaybackDiagnostics {
    pub device: Option<String>,
    pub sample_rate: Option<u32>,
    pub datagrams_per_sec: f32,
    pub muted_peer_count: u32,
    pub deafened: bool,
}
