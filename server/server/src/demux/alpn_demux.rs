use super::{BufferedHello, DemuxError, TlsAlert};
use common::structs::network::VoiceProtocol;
use rustls::server::Acceptor;
use socket2::{Domain, Protocol, Socket, Type};
use std::io::Cursor;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;

/// Splits the public TLS port between the API and the WebSocket voice transport.
///
/// Reads only the ClientHello, which is plaintext, then relays bytes to whichever backend
/// the ALPN names. No TLS is terminated here: each backend still completes its own
/// handshake on its own certificate material, so Rocket's client-certificate verification
/// and the WebSocket listener's are both untouched.
///
/// Owning the public socket is also what lets this server offer both transports on one
/// hostname and one certificate, with no proxy in front and nothing for an operator to
/// configure.
pub struct AlpnDemux {
    listen: SocketAddr,
    api: SocketAddr,
    websocket: Option<SocketAddr>,
}

impl AlpnDemux {
    // A peer that connects and sends nothing otherwise pins a socket and a task without
    // ever identifying itself. Both bounds are on the unauthenticated path, so they are
    // the only thing standing between an idle connection and an unbounded one.
    const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
    const MAX_HANDSHAKE_BYTES: usize = 16 * 1024;
    const READ_CHUNK: usize = 4096;
    const DIAL_TIMEOUT: Duration = Duration::from_secs(5);

    // How long startup waits for both backends to answer before giving up. Generous: a
    // cold Rocket runs migrations before it listens.
    const READINESS_TIMEOUT: Duration = Duration::from_secs(60);
    const READINESS_POLL: Duration = Duration::from_millis(200);

    pub fn new(listen: SocketAddr, api: SocketAddr, websocket: Option<SocketAddr>) -> Self {
        Self {
            listen,
            api,
            websocket,
        }
    }

    pub fn new_shared(
        listen: SocketAddr,
        api: SocketAddr,
        websocket: Option<SocketAddr>,
    ) -> Arc<Self> {
        Arc::new(Self::new(listen, api, websocket))
    }

    pub async fn start(&self) -> Result<(), DemuxError> {
        // Public traffic is not accepted until both backends answer. Without this a
        // request arriving during startup is relayed into a refused dial and returns a
        // TLS alert, which reads to the client as a broken server rather than one that is
        // still coming up.
        self.await_backends().await?;

        let listener = Self::bind_listener(self.listen)?;

        tracing::info!(
            listen = %self.listen,
            api = %self.api,
            websocket = ?self.websocket,
            "TLS demultiplexer listening"
        );

        loop {
            let (stream, peer) = match listener.accept().await {
                Ok(accepted) => accepted,
                Err(e) => {
                    tracing::warn!("accepting on the public TLS listener: {e}");
                    continue;
                }
            };

            let api = self.api;
            let websocket = self.websocket;
            tokio::spawn(async move {
                if let Err(e) = Self::handle_connection(stream, peer, api, websocket).await {
                    if e.is_routine() {
                        tracing::debug!(%peer, "demuxed connection ended: {e}");
                    } else {
                        tracing::warn!(%peer, "demuxed connection failed: {e}");
                    }
                }
            });
        }
    }

    async fn await_backends(&self) -> Result<(), DemuxError> {
        let deadline = tokio::time::Instant::now() + Self::READINESS_TIMEOUT;

        for backend in [Some(self.api), self.websocket].into_iter().flatten() {
            loop {
                match TcpStream::connect(backend).await {
                    Ok(_) => break,
                    Err(source) => {
                        if tokio::time::Instant::now() >= deadline {
                            return Err(DemuxError::BackendUnavailable {
                                addr: backend,
                                timeout: Self::READINESS_TIMEOUT,
                                source,
                            });
                        }
                        tokio::time::sleep(Self::READINESS_POLL).await;
                    }
                }
            }
        }

        Ok(())
    }

    async fn handle_connection(
        mut stream: TcpStream,
        peer: SocketAddr,
        api: SocketAddr,
        websocket: Option<SocketAddr>,
    ) -> Result<(), DemuxError> {
        let hello = match timeout(
            Self::HANDSHAKE_TIMEOUT,
            Self::read_client_hello(&mut stream, peer),
        )
        .await
        {
            Ok(Ok(hello)) => hello,
            Ok(Err(e)) => return Err(e),
            Err(_) => {
                Self::reject(&mut stream, TlsAlert::HANDSHAKE_FAILURE).await;
                return Err(DemuxError::HandshakeTimeout {
                    peer,
                    timeout: Self::HANDSHAKE_TIMEOUT,
                });
            }
        };

        let backend = match Self::backend_for(&hello, api, websocket) {
            Some(backend) => backend,
            None => {
                Self::reject(&mut stream, TlsAlert::NO_APPLICATION_PROTOCOL).await;
                return Err(DemuxError::VoiceTransportUnavailable { peer });
            }
        };

        let mut backend_stream = match timeout(Self::DIAL_TIMEOUT, TcpStream::connect(backend)).await
        {
            Ok(Ok(s)) => s,
            Ok(Err(source)) => {
                Self::reject(&mut stream, TlsAlert::INTERNAL_ERROR).await;
                return Err(DemuxError::BackendDial {
                    peer,
                    addr: backend,
                    source,
                });
            }
            Err(_) => {
                Self::reject(&mut stream, TlsAlert::INTERNAL_ERROR).await;
                return Err(DemuxError::BackendDialTimeout {
                    peer,
                    addr: backend,
                    timeout: Self::DIAL_TIMEOUT,
                });
            }
        };

        // Replay the handshake bytes this read off the wire, then get out of the way. The
        // backend sees a stream that begins exactly where the client began it.
        backend_stream
            .write_all(&hello.bytes)
            .await
            .map_err(|source| DemuxError::Relay {
                peer,
                addr: backend,
                source,
            })?;

        tokio::io::copy_bidirectional(&mut stream, &mut backend_stream)
            .await
            .map_err(|source| DemuxError::Relay {
                peer,
                addr: backend,
                source,
            })?;

        Ok(())
    }

    /// Which backend serves this hello.
    ///
    /// `None` only when the client explicitly asked for the voice transport and this
    /// server has none. **Everything else routes to the API**, including a hello with no
    /// ALPN at all: a browser cannot offer one, and the position feed is a browser socket.
    fn backend_for(
        hello: &BufferedHello,
        api: SocketAddr,
        websocket: Option<SocketAddr>,
    ) -> Option<SocketAddr> {
        if hello.offers(VoiceProtocol::ALPN) {
            return websocket;
        }

        Some(api)
    }

    async fn read_client_hello(
        stream: &mut TcpStream,
        peer: SocketAddr,
    ) -> Result<BufferedHello, DemuxError> {
        let mut acceptor = Acceptor::default();
        let mut bytes = Vec::with_capacity(Self::READ_CHUNK);
        let mut chunk = [0u8; Self::READ_CHUNK];

        loop {
            let read = stream
                .read(&mut chunk)
                .await
                .map_err(|source| DemuxError::Read { peer, source })?;

            if read == 0 {
                return Err(DemuxError::HandshakeIncomplete {
                    peer,
                    read: bytes.len(),
                });
            }

            bytes.extend_from_slice(&chunk[..read]);
            if bytes.len() > Self::MAX_HANDSHAKE_BYTES {
                return Err(DemuxError::HandshakeTooLarge {
                    peer,
                    read: bytes.len(),
                    limit: Self::MAX_HANDSHAKE_BYTES,
                });
            }

            // rustls consumes one record per call, so the chunk is drained into it rather
            // than handed over once.
            let mut cursor = Cursor::new(&chunk[..read]);
            while (cursor.position() as usize) < read {
                match acceptor.read_tls(&mut cursor) {
                    Ok(0) => break,
                    Ok(_) => {}
                    Err(source) => return Err(DemuxError::Read { peer, source }),
                }
            }

            match acceptor.accept() {
                Ok(Some(accepted)) => {
                    let alpn = accepted
                        .client_hello()
                        .alpn()
                        .map(|protocols| protocols.map(<[u8]>::to_vec).collect())
                        .unwrap_or_default();

                    return Ok(BufferedHello { bytes, alpn });
                }
                Ok(None) => continue,
                Err((source, mut alert)) => {
                    let mut encoded = Vec::new();
                    let _ = alert.write_all(&mut encoded);
                    if !encoded.is_empty() {
                        let _ = stream.write_all(&encoded).await;
                    }
                    let _ = stream.shutdown().await;
                    return Err(DemuxError::MalformedHello { peer, source });
                }
            }
        }
    }

    /// Sends a fatal alert and closes, so the peer sees a TLS-level refusal rather than a
    /// bare disconnect it would read as a network fault and retry.
    async fn reject(stream: &mut TcpStream, alert: &[u8]) {
        let _ = stream.write_all(alert).await;
        let _ = stream.shutdown().await;
    }

    /// Binds the public socket, explicitly dual-stack on a wildcard IPv6 address.
    ///
    /// Windows enables `IPV6_V6ONLY` by default, so a wildcard v6 bind there serves no
    /// IPv4 peer. Rocket 0.5 cannot be handed a pre-configured socket, which is why the
    /// HTTP listener used to fall back to the IPv4 wildcard and IPv6-only clients could
    /// not sign in at all. This socket is ours to configure, so the fallback is gone.
    fn bind_listener(addr: SocketAddr) -> Result<TcpListener, DemuxError> {
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
