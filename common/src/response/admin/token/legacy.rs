use serde::{Deserialize, Serialize};

// The pre-identifier scalar, stored in plaintext and shared by every mod. `configured`
// means it comes from the environment or config.hcl, in which case startup re-applies it
// and removing the row would achieve nothing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct LegacyTokenResponse {
    pub token: Option<String>,
    pub configured: bool,
}
