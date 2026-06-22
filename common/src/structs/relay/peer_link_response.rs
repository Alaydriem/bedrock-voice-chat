use serde::{Deserialize, Serialize};

// Server → operator/designated peer (Flow 2): the minted peer-link code. The
// requester redeems it at `/relay/peer-redeem` (presenting its bound endpoint)
// for the in-memory `server::`-CN peer cert.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct PeerLinkResponse {
    pub code: String,
}
