use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::relay_endpoint::RelayEndpoint;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LookupResponse {
    pub worlds: HashMap<String, Vec<RelayEndpoint>>,
}
