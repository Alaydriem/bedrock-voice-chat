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

impl PlayerGainSettings {
    /// What a player nobody holds an opinion about sounds like: unchanged and audible.
    ///
    /// The default is deliberately not silence. A lookup that misses — a wrong key, a device
    /// heard from before its settings arrived — then plays the speaker rather than dropping
    /// them, so a keying mistake is audible instead of a silent absence nobody can debug.
    pub fn unity() -> Self {
        Self {
            gain: 1.0,
            muted: false,
            last_seen: None,
        }
    }

    /// Whether the user has made a decision about this player, as opposed to merely having
    /// been near them.
    ///
    /// `last_seen` is deliberately not part of the answer. Proximity stamps every player who
    /// walks past, so counting a stamp as a decision would make the Players pane's default
    /// view the entire server. It also decides what the pruner may drop: a stamp expires, a
    /// decision never does.
    pub fn is_adjusted(&self) -> bool {
        self.gain != 1.0 || self.muted
    }
}
