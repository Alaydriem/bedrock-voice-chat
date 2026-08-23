use serde::{Deserialize, Serialize};

use crate::response::admin::relay::world::RelayWorld;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct RelayWorldsResponse {
    pub worlds: Vec<RelayWorld>,
}
