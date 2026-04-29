use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct GeneratedCodeResponse {
    pub code: String,
    pub expires_in_seconds: u64,
}
