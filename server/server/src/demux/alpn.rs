use common::curia;
use super::{BufferedHello, DemuxError, PeerClass, PublicListener, TlsAlert};
use common::structs::network::VoiceProtocol;
use rustls::server::Acceptor;
use std::io::Cursor;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
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
    // Read per connection rather than copied once: the API listener can land on a
    // different port than it was first given, and a copy taken at construction
    // would relay to an address nothing is listening on.
    api: super::api_bind::ApiBind,
    websocket: Option<SocketAddr>,
    // The plain-HTTP backend for the game mods. `None` unless the operator turned the
    // branch on. Only a loopback peer is ever relayed to it.
    plain: Option<super::api_bind::ApiBind>,
    // Reads true once the public socket accepts connections, for work that must not begin
    // before then. A watch rather than a notification, so any number of waiters see it,
    // including one that arrives late.
    listening: Arc<tokio::sync::watch::Sender<bool>>,
}

impl AlpnDemux {
    // The first byte of a TLS record is the handshake content type.
    const TLS_HANDSHAKE_RECORD: u8 = 0x16;

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

    pub fn new(
        listen: SocketAddr,
        api: super::api_bind::ApiBind,
        websocket: Option<SocketAddr>,
        plain: Option<super::api_bind::ApiBind>,
    ) -> Self {
        Self {
            listen,
            api,
            websocket,
            plain,
            listening: Arc::new(tokio::sync::watch::channel(false).0),
        }
    }

    /// Reads true once the public socket is bound and accepting.
    pub fn listening(&self) -> tokio::sync::watch::Receiver<bool> {
        self.listening.subscribe()
    }

    pub fn new_shared(
        listen: SocketAddr,
        api: super::api_bind::ApiBind,
        websocket: Option<SocketAddr>,
        plain: Option<super::api_bind::ApiBind>,
    ) -> Arc<Self> {
        Arc::new(Self::new(listen, api, websocket, plain))
    }

    pub async fn start(&self) -> Result<(), DemuxError> {
        // Public traffic is not accepted until both backends answer. Without this a
        // request arriving during startup is relayed into a refused dial and returns a
        // TLS alert, which reads to the client as a broken server rather than one that is
        // still coming up.
        self.await_backends().await?;

        let listener = PublicListener::bind(self.listen)?;

        curia::info!("TLS demultiplexer listening", { "listen": self.listen.to_string(), "api": self.api.addr().to_string(), "websocket": format!("{:?}", self.websocket), "plain": format!("{:?}", self.plain.as_ref().map(|plain| plain.addr())) });
        self.listening.send_replace(true);

        loop {
            let (stream, peer) = match listener.accept().await {
                Ok(accepted) => accepted,
                Err(e) => {
                    curia::warn!(format!("accepting on the public TLS listener: {e}"));
                    continue;
                }
            };

            let api = self.api.addr();
            let websocket = self.websocket;
            let plain = self.plain.as_ref().map(|plain| plain.addr());
            tokio::spawn(async move {
                if let Err(e) = Self::handle_connection(stream, peer, api, websocket, plain).await {
                    if e.is_routine() {
                        curia::debug!(format!("demuxed connection ended: {e}"), { "peer": peer.to_string() });
                    } else {
                        curia::warn!(format!("demuxed connection failed: {e}"), { "peer": peer.to_string() });
                    }
                }
            });
        }
    }

    async fn await_backends(&self) -> Result<(), DemuxError> {
        let deadline = tokio::time::Instant::now() + Self::READINESS_TIMEOUT;


        // The API address is resolved per attempt rather than once, because the
        // listener may still be settling on a port while this waits for it to
        // answer. The WebSocket listener is already bound by the time this runs,
        // so its address is fixed.
        let backends: [Option<Box<dyn Fn() -> SocketAddr + Send>>; 3] = [
            Some(Box::new({
                let api = self.api.clone();
                move || api.addr()
            })),
            self.websocket
                .map(|addr| Box::new(move || addr) as Box<dyn Fn() -> SocketAddr + Send>),
            self.plain.clone().map(|plain| {
                Box::new(move || plain.addr()) as Box<dyn Fn() -> SocketAddr + Send>
            }),
        ];

        for resolve in backends.into_iter().flatten() {
            loop {
                let backend = resolve();

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
        plain: Option<SocketAddr>,
    ) -> Result<(), DemuxError> {
        // One budget for identifying the peer, however it is spent. Peeking and reading the
        // hello under separate timeouts would let a peer hold the socket for twice as long.
        let deadline = tokio::time::Instant::now() + Self::HANDSHAKE_TIMEOUT;

        let first = match tokio::time::timeout_at(deadline, Self::peek_first_byte(&stream, peer))
            .await
        {
            Ok(Ok(first)) => first,
            Ok(Err(e)) => return Err(e),
            Err(_) => {
                Self::reject(&mut stream, TlsAlert::HANDSHAKE_FAILURE).await;
                return Err(DemuxError::HandshakeTimeout {
                    peer,
                    timeout: Self::HANDSHAKE_TIMEOUT,
                });
            }
        };

        if first != Self::TLS_HANDSHAKE_RECORD {
            return match plain {
                Some(plain) if PeerClass::is_loopback(peer.ip()) => {
                    Self::relay_plain(stream, peer, plain).await
                }
                // Nothing is written: a TLS alert or an HTTP error would answer a peer
                // that has no business on this port with more than it asked for.
                _ => {
                    let _ = stream.shutdown().await;
                    Err(DemuxError::PlainRefused { peer })
                }
            };
        }

        let hello = match tokio::time::timeout_at(
            deadline,
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

    // Looks at the first byte without consuming it, so whichever path follows reads the
    // stream from its start.
    async fn peek_first_byte(stream: &TcpStream, peer: SocketAddr) -> Result<u8, DemuxError> {
        let mut first = [0u8; 1];
        let peeked = stream
            .peek(&mut first)
            .await
            .map_err(|source| DemuxError::Read { peer, source })?;

        if peeked == 0 {
            return Err(DemuxError::HandshakeIncomplete { peer, read: 0 });
        }

        Ok(first[0])
    }

    // Relays a plain-HTTP connection from this machine to the mods' backend. No byte has
    // been consumed, so the backend sees the request exactly as the mod sent it.
    async fn relay_plain(
        mut stream: TcpStream,
        peer: SocketAddr,
        plain: SocketAddr,
    ) -> Result<(), DemuxError> {
        let mut backend_stream = match timeout(Self::DIAL_TIMEOUT, TcpStream::connect(plain)).await
        {
            Ok(Ok(s)) => s,
            Ok(Err(source)) => {
                return Err(DemuxError::BackendDial {
                    peer,
                    addr: plain,
                    source,
                });
            }
            Err(_) => {
                return Err(DemuxError::BackendDialTimeout {
                    peer,
                    addr: plain,
                    timeout: Self::DIAL_TIMEOUT,
                });
            }
        };

        tokio::io::copy_bidirectional(&mut stream, &mut backend_stream)
            .await
            .map_err(|source| DemuxError::Relay {
                peer,
                addr: plain,
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

}
