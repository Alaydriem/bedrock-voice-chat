use serde::{Deserialize, Serialize};

use super::relay_endpoint::RelayEndpoint;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct LookupRequest {
    pub caller: RelayEndpoint,
    pub hashed_worlds: Vec<String>,
    // Endpoint-control-proven token. The caller must control the
    // `caller` endpoint it claims — the relay enforces the same token binding
    // it requires for register, so an attacker who merely knows a world hash and
    // a registered member endpoint cannot pass `caller = victim_endpoint` to
    // enumerate peers.
    pub token: String,
}
