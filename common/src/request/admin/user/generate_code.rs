use serde::{Deserialize, Serialize};

use crate::Game;

fn default_ephemeral() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct GenerateCodeRequest {
    pub gamertag: String,
    pub game: Game,
    pub duration: u64,
    // Single-use when true (default); reusable until expiry when false.
    #[serde(default = "default_ephemeral")]
    pub ephemeral: bool,
}
