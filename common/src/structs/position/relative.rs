use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::PresenceKind;

/// Another player's position expressed relative to an observer.
///
/// Carries no absolute coordinate: a client learns that somebody is forty blocks to the
/// north-east, never where either of them is standing. What it does carry is who, because a
/// card needs a name and a distance needs somebody to belong to — and everyone in scope is
/// inside the observer's own visual range regardless.
///
/// The server decides who appears, using the same per-game rule voice routing uses at feed
/// range, so world, relay world, dimension and spectator state are all already accounted
/// for by the time an entry exists.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct RelativePosition {
    /// The certificate CN form, `game:gamertag`, with the gamertag's own casing. It is the
    /// identity channels, recordings and a player's colour all key on, and it is stable
    /// across sessions — which is what lets a card survive a reconnect.
    pub name: String,
    pub presence: PresenceKind,
    /// Bearing from the observer in degrees, 0-359, relative to their facing.
    pub bearing_deg: u16,
    /// Horizontal distance in blocks, rounded.
    pub distance: u16,
    /// Signed vertical offset in blocks, so the UI can distinguish someone
    /// above or below from someone alongside.
    pub elevation: i16,
}
