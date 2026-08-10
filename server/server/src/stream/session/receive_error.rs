/// Why a session stopped receiving.
///
/// Kept apart from `SendOutcome`, which describes one payload: this ends the session. The
/// distinction that matters to a reader of the log is whether the peer went away, which is
/// ordinary, or whether the transport itself could not be queried, which is not.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ReceiveError {
    /// The peer closed, or the connection failed underneath.
    #[error("the session closed: {detail}")]
    Closed { detail: String },

    /// The datagram provider could not be queried at all, which means the connection is
    /// already gone rather than that a receive failed.
    #[error("the transport could not be queried: {detail}")]
    Unavailable { detail: String },
}

impl ReceiveError {
    /// Whether this reads as a peer that left rather than something that broke.
    ///
    /// The input loop logs the two differently: a player closing their client is the
    /// common case and must not look like a fault.
    pub(crate) fn is_disconnect(&self) -> bool {
        match self {
            Self::Closed { detail } => {
                let lower = detail.to_ascii_lowercase();
                lower.contains("clos") || lower.contains("reset")
            }
            Self::Unavailable { .. } => false,
        }
    }
}
