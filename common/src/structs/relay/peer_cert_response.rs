use serde::{Deserialize, Serialize};

// The in-memory peer credential the acceptor issues to a mutually-proven peer.
// The acceptor returns the issued (cert, key) PEM pair plus its own CA PEM so
// the initiator can build an mTLS QUIC client that the acceptor's listener
// validates against that CA.
//
// Returning the private key over the wire is acceptable for v1 because this
// travels ONLY over the initiator's SPKI-pinned HTTPS channel to the acceptor
// (server↔server, pinned), the credential is never persisted, and it is scoped
// to a single peer identity gated on a completed mutual presence proof. A future
// hardening could have the initiator generate a CSR instead; v1 reuses
// `sign_peer_cert`'s issued pair.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct PeerCertResponse {
    pub ca_pem: String,
    pub cert_pem: String,
    pub key_pem: String,
}
