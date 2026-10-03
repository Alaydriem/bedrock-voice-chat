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
#[cfg_attr(
    feature = "openapi",
    schemars(
        description = "Everyone near the player right now. Each snapshot is complete and replaces the one before it."
    )
)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct PositionSnapshot {
    /// Monotonic per-session counter; lets the client discard reordered frames.
    #[cfg_attr(
        feature = "openapi",
        schemars(
            description = "Counts up by one for each snapshot on this socket. Ignore a snapshot with a lower number than the last one you used."
        )
    )]
    pub seq: u64,
    #[cfg_attr(
        feature = "openapi",
        schemars(
            description = "The other players near you, nearest first. Empty when nobody is near, or when you are not in the game."
        )
    )]
    pub positions: Vec<RelativePosition>,
}
