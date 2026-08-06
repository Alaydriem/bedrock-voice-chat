use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::structs::audio::NoiseGateStatus;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct MicDiagnostics {
    pub device: Option<String>,
    pub sample_rate: Option<u32>,
    // Read from the flag the capture path itself consults, combined with whether any
    // captured frame carried signal during the interval. Sampled over an interval rather
    // than instantaneously: at a 20 ms frame cadence a single reading lands on a
    // near-random frame and flickers.
    pub noise_gate: NoiseGateStatus,
    pub muted: bool,
    pub datagrams_per_sec: f32,
}
