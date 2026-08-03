use serde::{Deserialize, Serialize};
use ts_rs::TS;

// Measured after the noise gate and the mono fold, which is the level a speaker is
// actually transmitting rather than the level their microphone picked up.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct InputLevel {
    pub rms: f32,
    pub gate_open: bool,
}
