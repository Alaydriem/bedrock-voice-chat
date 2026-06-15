// Failure modes of peer-cert issuance, distinguished so the HTTP handler maps an
// unauthorized request to 403 and an internal signing failure to 500.
#[derive(Debug, thiserror::Error)]
pub enum PeerCertIssueError {
    #[error("peer {host}:{port} is not mutually proven for world {hashed_world} (cert issuance denied)")]
    NotProven {
        host: String,
        port: u16,
        hashed_world: String,
    },

    #[error("peer cert signing failed: {0}")]
    Signing(#[from] anyhow::Error),
}
