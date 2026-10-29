use serde::{Deserialize, Serialize};

/// A non-protected BVC channel as the bridge sees it at resync. `players` are gamertags.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct LinkedChannelView {
    pub id: String,
    pub name: String,
    pub players: Vec<String>,
    pub members_only: bool,
}
