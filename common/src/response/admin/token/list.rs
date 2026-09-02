use serde::{Deserialize, Serialize};

use super::AccessTokenRow;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct AccessTokenListResponse {
    pub tokens: Vec<AccessTokenRow>,
}
