/// Why a voice link could not be opened, or could not carry a packet.
///
/// The dial variants are what a player sees as a failed connection, so they name the step
/// that failed rather than rendering one opaque string: a certificate that will not parse
/// is the player's own credentials, and a refused upgrade is a server too old to serve
/// this transport. Those need different advice.
#[derive(Debug, thiserror::Error)]
pub enum VoiceLinkError {
    #[error("parsing the certificate chain for the voice transport")]
    ParseCertificates {
        #[source]
        source: std::io::Error,
    },

    #[error("parsing this player's private key for the voice transport")]
    ParseKey {
        #[source]
        source: std::io::Error,
    },

    #[error("this player's identity contains no private key")]
    MissingPrivateKey,

    #[error("adding the server's certificate authority to the trust roots")]
    TrustRoot {
        #[source]
        source: rustls::Error,
    },

    #[error("building the TLS configuration for the voice transport")]
    TlsConfig {
        #[source]
        source: rustls::Error,
    },

    #[error("{url} is not a usable WebSocket address")]
    InvalidUrl {
        url: String,
        #[source]
        source: tokio_tungstenite::tungstenite::Error,
    },

    /// The dial itself failed. Against a server with no voice listener this is where it
    /// surfaces — the ALPN is refused during the handshake — which is why the client needs
    /// no capability negotiation before trying.
    #[error("connecting the voice transport to {url}")]
    Connect {
        url: String,
        #[source]
        source: tokio_tungstenite::tungstenite::Error,
    },

    /// The send queue is full. The packet is lost, the session is not.
    #[error("the voice transport send queue is full")]
    SendQueueFull,

    #[error("the voice transport is closed")]
    Closed,

    #[error("sending on the QUIC connection")]
    Quic { detail: String },
}
