use serde::{Deserialize, Serialize};

use super::paired_peer::PairedPeer;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct PairedPeersResponse {
    pub peers: Vec<PairedPeer>,
}
