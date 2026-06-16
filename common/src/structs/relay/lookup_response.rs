use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::relay_endpoint::RelayEndpoint;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct LookupResponse {
    pub worlds: HashMap<String, Vec<RelayEndpoint>>,
}
