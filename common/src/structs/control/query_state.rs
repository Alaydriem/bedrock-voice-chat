use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct QueryState {
    pub id: String,
    pub muted: bool,
    pub deafened: bool,
    pub recording: bool,
    // Server-authoritative: overlaid from channel membership when the state is
    // read (`/api/state`), not trusted from the client's report. The nanoid of the
    // player's current group, or None.
    pub current_group: Option<String>,
}
