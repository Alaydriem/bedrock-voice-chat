/// Why the voice transport could not be opened.
///
/// The two are acted on differently and must not be collapsed into one string.
///
/// An unreachable endpoint is evidence about the network, so it demotes QUIC for the run and
/// falls back to the WebSocket transport.
///
/// A certificate fault is evidence about this device's credentials. Every listener validates
/// the client certificate against the same CA, so the other transport will repeat the rejection
/// rather than route around it — which is why this neither falls back nor demotes QUIC on a
/// verdict about UDP that nothing established. Either transport can raise it; whether it
/// actually ends the session is decided by the credential probe at the command boundary.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ConnectFailure {
    #[error("the server's certificate was rejected: {detail}")]
    Certificate { detail: String },

    #[error("{detail}")]
    Unreachable { detail: String },
}

impl ConnectFailure {
    pub(crate) fn is_certificate(&self) -> bool {
        matches!(self, Self::Certificate { .. })
    }

    pub(crate) fn detail(&self) -> &str {
        match self {
            Self::Certificate { detail } | Self::Unreachable { detail } => detail,
        }
    }
}
