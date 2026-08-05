use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct PlayerGainSettings {
    pub gain: f32,
    pub muted: bool,
    /// Unix milliseconds when this player was last near you, absent for an entry written
    /// before it was recorded.
    ///
    /// This store is already the list of players a device holds an opinion about, so stamping
    /// it makes the same store answer "who have I been around lately" — which is what the
    /// settings pane needs, with no second list to keep in step with this one.
    ///
    /// `f64` rather than `u64` because this store is written from TypeScript as well, and
    /// ts-rs maps a `u64` to `bigint` — which `JSON.stringify` refuses, so every save would
    /// throw. A double holds epoch milliseconds exactly for the next quarter of a million
    /// years, and it is what `Date.now()` returns anyway.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_seen: Option<f64>,
}
