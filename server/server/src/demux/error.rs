use std::net::SocketAddr;

/// Why the TLS demultiplexer refused, dropped or could not start a connection.
///
/// Concrete variants rather than one opaque string, because the three fatal cases are
/// operationally different and an operator has to tell them apart from a log line: a host
/// that will not give a dual-stack socket needs a kernel setting, a port already in use
/// needs a different process stopped, and a backend that never answered means the server
/// came up broken.
#[derive(Debug, thiserror::Error)]
pub enum DemuxError {
    #[error("this host refuses a dual-stack socket on {addr}, so IPv4 peers could not reach it")]
    NotDualStack {
        addr: SocketAddr,
        #[source]
        source: std::io::Error,
    },

    #[error("binding the public TLS listener on {addr}")]
    Bind {
        addr: SocketAddr,
        #[source]
        source: std::io::Error,
    },

    #[error("backend {addr} never answered within {timeout:?}")]
    BackendUnavailable {
        addr: SocketAddr,
        timeout: std::time::Duration,
        #[source]
        source: std::io::Error,
    },

    /// The peer opened a connection and never completed a ClientHello. Ordinary on a
    /// public port — scanners do it constantly — so this is logged at debug, not warned.
    #[error("no complete ClientHello from {peer} within {timeout:?}")]
    HandshakeTimeout {
        peer: SocketAddr,
        timeout: std::time::Duration,
    },

    #[error("{peer} sent {read} handshake bytes without completing a ClientHello (limit {limit})")]
    HandshakeTooLarge {
        peer: SocketAddr,
        read: usize,
        limit: usize,
    },

    #[error("{peer} closed after {read} bytes, before a complete ClientHello")]
    HandshakeIncomplete { peer: SocketAddr, read: usize },

    #[error("malformed ClientHello from {peer}")]
    MalformedHello {
        peer: SocketAddr,
        #[source]
        source: rustls::Error,
    },

    /// The client asked for the voice transport on a server that has none. Distinct from
    /// every other refusal: it is the client's signal to stop trying this transport.
    #[error("{peer} asked for the voice transport, which is not enabled here")]
    VoiceTransportUnavailable { peer: SocketAddr },

    #[error("dialling backend {addr} for {peer}")]
    BackendDial {
        peer: SocketAddr,
        addr: SocketAddr,
        #[source]
        source: std::io::Error,
    },

    #[error("dialling backend {addr} for {peer} timed out after {timeout:?}")]
    BackendDialTimeout {
        peer: SocketAddr,
        addr: SocketAddr,
        timeout: std::time::Duration,
    },

    #[error("relaying {peer} to {addr}")]
    Relay {
        peer: SocketAddr,
        addr: SocketAddr,
        #[source]
        source: std::io::Error,
    },

    #[error("could not reserve a loopback port after {attempts} attempts")]
    PortReservation {
        attempts: usize,
        #[source]
        source: std::io::Error,
    },

    #[error("reading from {peer}")]
    Read {
        peer: SocketAddr,
        #[source]
        source: std::io::Error,
    },
}

impl DemuxError {
    /// Whether this ended a connection that had already identified itself.
    ///
    /// An unfinished or malformed handshake on a public port is background noise from
    /// scanners; logging every one at warn would bury the failures that matter.
    pub fn is_routine(&self) -> bool {
        matches!(
            self,
            Self::HandshakeTimeout { .. }
                | Self::HandshakeTooLarge { .. }
                | Self::HandshakeIncomplete { .. }
                | Self::MalformedHello { .. }
                | Self::Read { .. }
                | Self::Relay { .. }
        )
    }
}
