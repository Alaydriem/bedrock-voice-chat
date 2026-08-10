use std::net::SocketAddr;

/// Why the WebSocket voice listener could not start, or refused a connection.
///
/// The startup variants and the per-connection variants are deliberately one type: an
/// operator reading a log needs "this server cannot serve voice at all" to look different
/// from "one client was turned away", and `is_startup` is what makes that difference
/// checkable rather than a matter of reading the message.
#[derive(Debug, thiserror::Error)]
pub enum WebSocketListenerError {
    #[error("reading {path}")]
    ReadFile {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("parsing certificates in {path}")]
    ParseCertificates {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("no private key found in {path}")]
    MissingPrivateKey { path: String },

    #[error("adding {path} to the client trust roots")]
    TrustRoot {
        path: String,
        #[source]
        source: rustls::Error,
    },

    #[error("building the client certificate verifier")]
    ClientVerifier {
        #[source]
        source: rustls::server::VerifierBuilderError,
    },

    #[error("building the WebSocket TLS configuration")]
    TlsConfig {
        #[source]
        source: rustls::Error,
    },

    #[error("binding the WebSocket voice listener")]
    Bind {
        #[source]
        source: std::io::Error,
    },

    #[error("TLS handshake from {peer}")]
    Handshake {
        peer: SocketAddr,
        #[source]
        source: std::io::Error,
    },

    /// The certificate verified against the CA but did not name a player this server will
    /// serve — a peer CN, or a name that classifies as neither.
    #[error("{peer} presented no usable player identity")]
    UnusableIdentity { peer: SocketAddr },

    #[error("WebSocket upgrade from {peer}")]
    Upgrade {
        peer: SocketAddr,
        #[source]
        source: tokio_tungstenite::tungstenite::Error,
    },
}

impl WebSocketListenerError {
    /// Whether this stopped the listener rather than one connection.
    pub fn is_startup(&self) -> bool {
        !matches!(
            self,
            Self::Handshake { .. } | Self::UnusableIdentity { .. } | Self::Upgrade { .. }
        )
    }
}
