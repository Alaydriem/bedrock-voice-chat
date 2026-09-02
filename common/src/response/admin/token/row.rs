use serde::{Deserialize, Serialize};

// One issued credential as an operator sees it. Never carries the secret.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct AccessTokenRow {
    pub id: String,
    pub created_at: i64,
    pub revoked_at: Option<i64>,
}
