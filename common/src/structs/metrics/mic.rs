use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct MicDiagnostics {
    pub device: Option<String>,
    pub sample_rate: Option<u32>,
    // Derived from whether any captured frame carried signal during the interval, not
    // sampled instantaneously: at a 20 ms frame cadence a single reading lands on a
    // near-random frame and flickers.
    pub gate_open: bool,
    pub muted: bool,
    pub datagrams_per_sec: f32,
}
