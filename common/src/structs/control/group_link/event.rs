use serde::{Deserialize, Serialize};

use super::GroupLinkEventKind;

/// One BVC channel change, in the order the server applied it.
///
/// `player` is the gamertag for `Join` and `Leave`, and the creator for `Create`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct GroupLinkEvent {
    pub seq: u64,
    pub kind: GroupLinkEventKind,
    pub channel: String,
    pub channel_name: Option<String>,
    pub player: Option<String>,
    pub members_only: bool,
}
