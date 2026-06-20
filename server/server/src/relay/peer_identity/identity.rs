// An in-memory, ephemeral server-peer credential produced by redeeming a code.
// Never persisted; valid only for the peer connection it bootstraps. The
// `endpoint`/`world` are the link→world binding the authorization gate enforces.
#[derive(Debug, Clone)]
pub struct RedeemedPeerIdentity {
    // The recipient's `host:port` endpoint (the cert CN carries `server::{endpoint}`).
    pub endpoint: String,
    // The relay world this peer is authorized to relay (in and out).
    pub world: String,
    pub ca_pem: String,
    pub cert_pem: String,
    pub key_pem: String,
}
