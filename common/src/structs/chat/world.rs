use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::mode::ChatMode;

/// A world this player has been seen in, and whether chat works there right now.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ChatWorld {
    /// Never rendered. A random UUID on the mod path and a derived hash on the proxy path,
    /// polymorphic in length, and meaningless to a person either way.
    pub world_uuid: String,
    /// The only user-facing label.
    pub world_name: String,
    pub last_seen: u64,
    /// The world is being hosted: position ingress is recent.
    pub active: bool,
    /// A mod chat channel is registered for it right now. A world can be active and
    /// unavailable — positions flowing while the chat socket is down.
    pub available: bool,
    pub mode: ChatMode,
}
