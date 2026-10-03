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
#[cfg_attr(
    feature = "openapi",
    schemars(
        description = "One other player, described by where they are compared to you. No coordinates are sent."
    )
)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct RelativePosition {
    /// The certificate CN form, `game:gamertag`, with the gamertag's own casing. It is the
    /// identity channels, recordings and a player's colour all key on, and it is stable
    /// across sessions — which is what lets a card survive a reconnect.
    #[cfg_attr(
        feature = "openapi",
        schemars(
            description = "The player, as game:gamertag, for example minecraft:Steve. It stays the same when they reconnect."
        )
    )]
    pub name: String,
    #[cfg_attr(
        feature = "openapi",
        schemars(description = "Whether this player is connected to voice chat.")
    )]
    pub presence: PresenceKind,
    /// Bearing from the observer in degrees, 0-359, relative to their facing.
    #[cfg_attr(
        feature = "openapi",
        schemars(
            description = "Direction to the player in degrees, clockwise from the way you face. 0 is ahead, 90 is to your right, 180 is behind you, 270 is to your left."
        )
    )]
    pub bearing_deg: u16,
    /// Horizontal distance in blocks, rounded.
    #[cfg_attr(
        feature = "openapi",
        schemars(
            description = "Distance to the player in blocks, measured flat along the ground and rounded."
        )
    )]
    pub distance: u16,
    /// Signed vertical offset in blocks, so the UI can distinguish someone
    /// above or below from someone alongside.
    #[cfg_attr(
        feature = "openapi",
        schemars(
            description = "Height difference in blocks. Positive when the player is above you, negative when below."
        )
    )]
    pub elevation: i16,
}
