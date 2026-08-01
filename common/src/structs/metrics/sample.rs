use serde::{Deserialize, Serialize};
use ts_rs::TS;

// One entry in the rolling history. `rtt_ms` is absent rather than zero when no round trip
// has been measured yet, because a zero would read as an impossibly perfect link.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct LinkSample {
    pub at_ms: u64,
    pub rtt_ms: Option<u32>,
    pub uplink_loss_pct: f32,
    pub worst_concealment_pct: f32,
}
