use serde::{Deserialize, Serialize};

use crate::structs::control::ClientAction;

/// A control action as a game server submits it, with what the game server knows about the
/// actor beyond their name.
///
/// `player_uuid` lives here and not on [`ClientAction`]: that type also travels in a postcard
/// datagram, where a new field changes the wire.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct ClientActionRequest {
    #[serde(flatten)]
    pub action: ClientAction,
    /// The actor's platform UUID. Absent from the BDS addon and from mods that predate the
    /// field, in which case an alias on the actor's name is followed as it always was.
    #[serde(default)]
    pub player_uuid: Option<String>,
}
