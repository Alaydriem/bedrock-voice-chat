use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{ConnectTargetKind, Glyph, ServerGlyph};

/// The world this client is connected to right now.
///
/// Rides on every state frame so a controller draws its toggle from the same push it
/// already subscribes to, rather than polling a second endpoint that would go stale the
/// moment the user disconnects from the app itself.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ActiveConnection {
    pub id: String,
    pub name: String,
    pub kind: ConnectTargetKind,
    /// Derived from `name`, matching the entry in a `targets` response for the same world.
    pub glyph: Glyph,
}

impl ActiveConnection {
    /// Attaches the derived glyph, so no construction site has to remember to.
    pub fn new(id: String, name: String, kind: ConnectTargetKind) -> Self {
        let glyph = ServerGlyph::of(&name);
        Self {
            id,
            name,
            kind,
            glyph,
        }
    }
}
