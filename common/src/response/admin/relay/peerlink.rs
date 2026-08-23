use serde::{Deserialize, Serialize};

// This server's peer link, and the node id inside it.
//
// The link is the only value an operator pastes. `node_id` is carried beside it
// so the same key can be matched against the log lines that name a peer, which
// print the key rather than the whole link.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct PeerLinkResponse {
    pub peerlink: String,
    pub node_id: String,
}
