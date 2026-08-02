use serde::{Deserialize, Serialize};

/// Another player's position expressed relative to an observer, anonymised.
///
/// Carries no gamertag and no absolute coordinate: a client learns that
/// something is closing from the north-east, not who it is or where they both
/// stand. That is what keeps the feed from becoming a player-location API.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelativePosition {
    /// Stable only within one WebSocket session, so the UI can track an entry
    /// across frames and animate it. Meaningless across sessions and not
    /// reversible to an identity.
    pub handle: u32,
    /// Bearing from the observer in degrees, 0-359, relative to their facing.
    pub bearing_deg: u16,
    /// Horizontal distance in blocks, rounded.
    pub distance: u16,
    /// Signed vertical offset in blocks, so the UI can distinguish someone
    /// above or below from someone alongside.
    pub elevation: i16,
}
