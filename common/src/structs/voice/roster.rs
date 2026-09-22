use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::VoiceMember;

/// Everyone this client can hear, as one frame.
///
/// Keyed the same way `LevelSnapshot` keys its peers, because a consumer joins the two by name:
/// this frame decides which meters exist, and the level frame drives them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct VoiceRoster {
    /// This client's own canonical identity. Never in `members`.
    pub own: String,
    pub members: Vec<VoiceMember>,
}

impl VoiceRoster {
    pub fn empty(own: String) -> Self {
        Self {
            own,
            members: Vec::new(),
        }
    }
}
