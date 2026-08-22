use serde::{Deserialize, Serialize};

fn default_connections() -> u32 {
    0
}

fn default_reconnect_grace() -> u64 {
    60
}

/// How many concurrent voice sessions this server admits.
///
/// An operator sizing control. `connections = 0` admits everyone, which is what every
/// deployment that predates this block reads as.
#[derive(Serialize, Deserialize, Debug, Clone, schemars::JsonSchema)]
pub struct VoiceLimits {
    // Maximum concurrent voice sessions, counted by canonical player identity rather than
    // by connection: one player reconnecting holds two connections for a moment and must
    // not consume two slots. 0 is unlimited.
    #[serde(default = "default_connections")]
    pub connections: u32,
    // Seconds a departed identity keeps its slot, so a player whose client closed the
    // socket cleanly is not displaced by somebody who arrived while they were returning.
    #[serde(default = "default_reconnect_grace")]
    pub reconnect_grace: u64,
}

impl Default for VoiceLimits {
    fn default() -> Self {
        Self {
            connections: default_connections(),
            reconnect_grace: default_reconnect_grace(),
        }
    }
}
