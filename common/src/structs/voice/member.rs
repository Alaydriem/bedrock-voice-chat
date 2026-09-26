use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::structs::players::PlayerSource;

/// One person this client can hear.
///
/// Membership is decided before a member reaches here: a name is present only while it holds a
/// live voice connection. A player known from the position feed alone — in the world, with no
/// BVC client — is not audible and never appears.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct VoiceMember {
    /// Canonical `game:gamertag`, the same key `LevelSnapshot.peers` uses.
    pub name: String,
    /// Why this member is audible. Never empty, and both are possible at once.
    pub sources: Vec<PlayerSource>,
    /// Public avatar URL, absent until resolved or when the player has none.
    pub gamerpic: Option<String>,
}
