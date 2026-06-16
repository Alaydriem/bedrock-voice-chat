use serde::{Deserialize, Serialize};

// The initiator's request to the acceptor for an in-memory peer client cert
// The acceptor signs a leaf cert for the requesting
// peer's `host:port` identity ONLY when that peer is mutually presence-proven
// for `hashed_world`; otherwise it is denied (default deny).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct PeerCertRequest {
    pub host: String,
    pub port: u16,
    pub hashed_world: String,
}
