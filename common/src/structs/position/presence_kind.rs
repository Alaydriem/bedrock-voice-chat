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
#[cfg_attr(
    feature = "openapi",
    schemars(description = "Whether a player is connected to voice chat.")
)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
#[serde(rename_all = "lowercase")]
pub enum PresenceKind {
    /// Connected to voice: they can hear you.
    #[cfg_attr(
        feature = "openapi",
        schemars(description = "Connected to voice chat. They can hear you.")
    )]
    Voice,
    /// In the world with no voice connection.
    #[cfg_attr(
        feature = "openapi",
        schemars(
            description = "In the game but not connected to voice chat. They cannot hear you."
        )
    )]
    Game,
}
