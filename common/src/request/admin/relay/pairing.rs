use serde::{Deserialize, Serialize};

// Asks the server to mint a single-use pairing code.
//
// `ttl_secs` is optional so a caller that has no opinion gets the service's default rather
// than having to restate it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct PairingRequest {
    pub label: String,
    pub ttl_secs: Option<u64>,
}
