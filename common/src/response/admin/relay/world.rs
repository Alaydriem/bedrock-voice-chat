use serde::{Deserialize, Serialize};

// One relay world and how many players are currently in it.
//
// The count is presence, not history: it is drawn from a cache whose entries
// expire, so a world with no live players is absent rather than reported as
// zero.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct RelayWorld {
    pub world: String,
    pub players: usize,
}
