use super::DemuxError;
use socket2::{Domain, Protocol, Socket, Type};
use std::net::SocketAddr;
use tokio::net::TcpListener;

/// The server's public TCP socket, configured the same way for every owner of the port.
pub struct PublicListener;

impl PublicListener {
    /// Binds the public socket, explicitly dual-stack on a wildcard IPv6 address.
    ///
    /// Windows enables `IPV6_V6ONLY` by default, so a wildcard v6 bind there serves no
    /// IPv4 peer. Rocket 0.5 cannot be handed a pre-configured socket, which is why the
    /// HTTP listener used to fall back to the IPv4 wildcard and IPv6-only clients could
    /// not sign in at all. This socket is ours to configure, so the fallback is gone.
    pub fn bind(addr: SocketAddr) -> Result<TcpListener, DemuxError> {
        let domain = if addr.is_ipv6() {
            Domain::IPV6
        } else {
            Domain::IPV4
        };

        let socket = Socket::new(domain, Type::STREAM, Some(Protocol::TCP))
            .map_err(|source| DemuxError::Bind { addr, source })?;

        if addr.is_ipv6() {
            socket
                .set_only_v6(false)
                .map_err(|source| DemuxError::NotDualStack { addr, source })?;
        }

        // A connection this server closed holds the port in TIME_WAIT for about a minute, and
        // without SO_REUSEADDR a restart in that window cannot bind it. tokio's
        // `TcpListener::bind` sets this on Unix; a socket2 socket does not. Not on Windows,
        // where SO_REUSEADDR lets another process bind a port that is in use.
        #[cfg(unix)]
        socket
            .set_reuse_address(true)
            .map_err(|source| DemuxError::Bind { addr, source })?;

        socket
            .bind(&addr.into())
            .map_err(|source| DemuxError::Bind { addr, source })?;
        socket
            .listen(1024)
            .map_err(|source| DemuxError::Bind { addr, source })?;
        socket
            .set_nonblocking(true)
            .map_err(|source| DemuxError::Bind { addr, source })?;

        TcpListener::from_std(socket.into()).map_err(|source| DemuxError::Bind { addr, source })
    }
}
