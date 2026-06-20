use serde::{Deserialize, Serialize};

// Operator/designated-peer → server (Flow 2): "issue me a peer-link code for
// `hashed_world`, bound to `{recipient_host}:{recipient_port}`." The requester is
// mTLS-authenticated (a cert signed by this server's CA), so — unlike Flow 1 — the
// code is returned directly in the response rather than injected through the
// realm. The minted code is single-use, recipient-bound, world-scoped, and
// short-TTL; redeeming it yields a `server::`-CN peer cert.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct PeerLinkRequest {
    pub hashed_world: String,
    pub recipient_host: String,
    pub recipient_port: u16,
}
