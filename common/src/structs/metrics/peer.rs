use serde::{Deserialize, Serialize};
use ts_rs::TS;

// Per-speaker receive counters. Underruns with no drops means that speaker stopped sending;
// drops mean the network. Separating those two is the whole reason this type exists.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct PeerDiagnostics {
    pub name: String,
    pub underruns: u64,
    pub overflow_drops: u64,
    pub ooo_drops: u64,
    pub plc_frames: u64,
    pub silence_frames: u64,
    pub frames_decoded: u64,
    pub ring_len: u32,
    pub capacity: u32,
    pub warmup_needed: u32,
    pub quality_score: f64,
    // What fraction of the audio played for this speaker was fabricated rather than decoded.
    // Concealment is what a listener actually hears, and unlike a loss percentage it is derivable
    // from what this client can observe — see the note on `LinkDiagnostics`.
    pub concealment_pct: f32,
    pub buffer_ms: u32,
}
