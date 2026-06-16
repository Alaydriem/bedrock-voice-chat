use serde::{Deserialize, Serialize};

use super::relay_endpoint::RelayEndpoint;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct RegisterRequest {
    pub hashed_world: String,
    pub endpoint: RelayEndpoint,
    pub ttl_secs: u32,
    // Endpoint-control proof bearer obtained from `/relay/challenge`. The relay
    // accepts a register only when this token was issued for
    // `endpoint` and that endpoint served the relay's nonce back. Defaulted so
    // older encodings still deserialize, but the relay rejects an empty/unproven
    // token (default deny).
    #[serde(default)]
    pub token: String,
}
