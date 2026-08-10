use common::s2n_quic::provider::datagram::default::DatagramError;
use core::fmt;

/// Why an inbound receive ended.
///
/// The concrete `DatagramError` is preserved rather than flattened into `anyhow`, because
/// a server-initiated close carries an application error code that only the typed value
/// exposes — every connection-level failure renders as the same opaque string through
/// `Display`.
pub(crate) enum RecvFailure {
    /// The QUIC connection itself failed or was closed by the peer.
    Datagram(DatagramError),
    /// The datagram provider could not be queried, i.e. the connection is already gone.
    Query(String),
    /// The WebSocket ended. There is no application error code to read: a WebSocket
    /// session that reaches the receive loop has already been authenticated, so a refusal
    /// arrives as a failed upgrade rather than as a close on a live session.
    Closed(String),
}

impl fmt::Display for RecvFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RecvFailure::Datagram(e) => write!(f, "{}", e),
            RecvFailure::Query(e) => write!(f, "{}", e),
            RecvFailure::Closed(e) => write!(f, "{}", e),
        }
    }
}
