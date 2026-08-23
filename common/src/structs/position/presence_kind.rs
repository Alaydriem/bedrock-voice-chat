use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Whether a player near you is on voice at all.
///
/// Somebody in the world without BVC is still worth drawing. They are standing in front of
/// you and nothing you say reaches them, which is the most common confusion a proximity
/// voice product produces — and omitting them from the feed makes them indistinguishable
/// from nobody being there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
#[serde(rename_all = "lowercase")]
pub enum PresenceKind {
    /// Connected to voice: they can hear you.
    Voice,
    /// In the world with no voice connection.
    Game,
}
