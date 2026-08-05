use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::relative::RelativePosition;

/// A complete picture of what the observer can see, not a delta.
///
/// Each snapshot supersedes the last, so a dropped or delayed frame costs one
/// animation step rather than leaving the UI permanently wrong. Approach,
/// departure and steady state are derived client-side by diffing `name`
/// across consecutive snapshots.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct PositionSnapshot {
    /// Monotonic per-session counter; lets the client discard reordered frames.
    pub seq: u64,
    pub positions: Vec<RelativePosition>,
}
