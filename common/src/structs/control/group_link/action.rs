use serde::{Deserialize, Serialize};

/// A change the Simple Voice Chat bridge asks for on the BVC side of a linked group.
///
/// Joins and leaves are not here: the bridge sends those as `ClientAction`s, as any in-game
/// actor does.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub enum GroupLinkAction {
    /// The BVC side of a group created in SVC. `creator` is the in-game name, resolved like a
    /// `ClientAction` actor. Returns the new channel id.
    CreateLinked {
        creator: String,
        name: String,
        members_only: bool,
    },
    /// The SVC side ended. Refused for a protected channel.
    Close { channel: String },
    /// The channels the bridge still links. Keeps Java-only groups from the reaper.
    Lease { channels: Vec<String> },
}
