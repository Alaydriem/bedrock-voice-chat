use common::curia;
use super::{WebSocketListenerError, WsLink};
use common::structs::network::VoiceProtocol;
use crate::stream::session::{SessionLink, SessionSpawner, WebSocketDeviceId};
use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::server::WebPkiClientVerifier;
use rustls::{RootCertStore, ServerConfig};
use std::collections::VecDeque;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_rustls::{TlsAcceptor, server::TlsStream};
use tokio_tungstenite::tungstenite::Message;

/// Serves the WebSocket voice transport for clients that cannot get a QUIC datagram out.
///
/// Authentication is the client certificate and nothing else — the same certificate the
/// QUIC listener checks, verified against the same CA, read for the same Common Name. A
/// session that reaches the spawner is therefore indistinguishable from a QUIC one.
///
/// Binds loopback: the public port belongs to the demultiplexer, which relays here when a
/// client offers the voice protocol.
pub struct WebSocketListener {
    listener: TcpListener,
    acceptor: TlsAcceptor,
    spawner: Arc<SessionSpawner>,
    /// The same instance the QUIC handshake and the HTTP guard hold, so a revocation is
    /// visible on every ingress rather than only the one that wrote it.
    authorization: Arc<crate::services::SessionAuthorizationService>,
    database: Arc<sea_orm::DatabaseConnection>,
    devices: Arc<WebSocketDeviceId>,
    /// Counts connections refused before they became sessions. Installed after
    /// construction because the metrics service is built later in startup than this
    /// listener's certificate material is read.
    metrics: Option<Arc<crate::services::MetricsService>>,
}

impl WebSocketListener {
    // The write pump's own queue. Under backpressure it discards the OLDEST frame: late
    // audio has already missed its moment, so keeping it in front of fresher audio buys
    // nothing and costs latency for the rest of the session.
    const WRITE_QUEUE_DEPTH: usize = 256;

    // Handoff between the routing layer and the write pump. Deliberately shallower than
    // the pump's queue, because it is a handoff rather than a buffer.
    const WRITE_HANDOFF_DEPTH: usize = 64;

    // Inbound frames waiting to be parsed. One frame is one packet, so this is measured in
    // packets exactly like the QUIC datagram receive capacity.
    const READ_QUEUE_DEPTH: usize = 256;

    // `MAX_DATAGRAM_SIZE` is enforced when a packet is encoded, which is AFTER a receiver
    // has already buffered whatever arrived. tokio-tungstenite defaults to 64 MiB, so
    // without this an unauthenticated-sized frame could be buffered before anything looks
    // at it. One packet per frame means the packet bound is the frame bound.
    const MAX_FRAME_BYTES: usize = 64 * 1024;

    /// Binds loopback on an ephemeral port and reports where.
    ///
    /// The port is never configured and never advertised — the demultiplexer is told it
    /// directly — so nothing outside this process needs to know it exists.
    pub(crate) async fn bind(
        certificate_path: &str,
        key_path: &str,
        ca_path: &str,
        spawner: Arc<SessionSpawner>,
        authorization: Arc<crate::services::SessionAuthorizationService>,
        database: Arc<sea_orm::DatabaseConnection>,
    ) -> Result<(Self, SocketAddr), WebSocketListenerError> {
        // Named explicitly rather than taken from the process default: whether one is
        // installed depends on which other component initialised rustls first, and
        // `ServerConfig::builder` panics rather than erroring when none is.
        let mut config = ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::aws_lc_rs::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .map_err(|source| WebSocketListenerError::TlsConfig { source })?
        .with_client_cert_verifier(Self::client_verifier(ca_path)?)
            .with_single_cert(
                Self::load_certificates(certificate_path)?,
                Self::load_key(key_path)?,
            )
            .map_err(|source| WebSocketListenerError::TlsConfig { source })?;

        config.alpn_protocols = vec![VoiceProtocol::ALPN.to_vec()];

        let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))
            .await
            .map_err(|source| WebSocketListenerError::Bind { source })?;
        let addr = listener
            .local_addr()
            .map_err(|source| WebSocketListenerError::Bind { source })?;

        Ok((
            Self {
                listener,
                acceptor: TlsAcceptor::from(Arc::new(config)),
                spawner,
                authorization,
                database,
                devices: Arc::new(WebSocketDeviceId::new()),
                metrics: None,
            },
            addr,
        ))
    }

    /// Installs the metrics service, so a refusal here is visible to the operator watching
    /// the Prometheus endpoint rather than only to the client that was refused.
    pub fn set_metrics(&mut self, metrics: Arc<crate::services::MetricsService>) {
        self.metrics = Some(metrics);
    }

    pub async fn start(self) -> Result<(), WebSocketListenerError> {
        let bind = self
            .listener
            .local_addr()
            .map_err(|source| WebSocketListenerError::Bind { source })?;
        curia::info!("WebSocket voice listener started", { "bind": bind.to_string() });

        loop {
            let (stream, peer) = match self.listener.accept().await {
                Ok(accepted) => accepted,
                Err(e) => {
                    curia::warn!(format!("accepting on the WebSocket listener: {e}"));
                    continue;
                }
            };

            let acceptor = self.acceptor.clone();
            let spawner = self.spawner.clone();
            let authorization = self.authorization.clone();
            let database = self.database.clone();
            let device = self.devices.next();
            let metrics = self.metrics.clone();

            tokio::spawn(async move {
                match Self::serve(
                    stream,
                    peer,
                    acceptor,
                    spawner,
                    authorization,
                    database,
                    device,
                )
                .await
                {
                    Ok(()) => {}
                    Err(e) => {
                        // Everything that fails before `serve` reaches the spawner is a
                        // refusal: a TLS handshake that did not complete, a certificate
                        // naming no player, an upgrade that never finished.
                        if let Some(metrics) = metrics {
                            metrics.record_websocket_rejection();
                        }
                        curia::debug!(format!("websocket session ended: {e}"), { "peer": peer.to_string(), "device": device });
                    }
                }
            });
        }
    }

    async fn serve(
        stream: TcpStream,
        peer: SocketAddr,
        acceptor: TlsAcceptor,
        spawner: Arc<SessionSpawner>,
        authorization: Arc<crate::services::SessionAuthorizationService>,
        database: Arc<sea_orm::DatabaseConnection>,
        device: u64,
    ) -> Result<(), WebSocketListenerError> {
        let tls = acceptor
            .accept(stream)
            .await
            .map_err(|source| WebSocketListenerError::Handshake { peer, source })?;

        let leaf_der = Self::presented_leaf(&tls)
            .ok_or(WebSocketListenerError::UnusableIdentity { peer })?;
        let fingerprint =
            crate::services::SessionAuthorizationService::fingerprint(&leaf_der);

        // Authorized the same way the QUIC handshake is, through the same service, so
        // neither transport admits a population the other refuses.
        let player = authorization
            .authorize(database.as_ref(), &leaf_der)
            .await
            .map_err(|reason| {
                curia::warn!(format!("Refusing WebSocket session: {}", reason), { "peer": peer.to_string() });
                WebSocketListenerError::UnusableIdentity { peer }
            })?;

        let identity = player
            .gamertag
            .as_ref()
            .map(|gamertag| player.game.membership_key(gamertag).to_string())
            .ok_or(WebSocketListenerError::UnusableIdentity { peer })?;

        let mut config = tokio_tungstenite::tungstenite::protocol::WebSocketConfig::default();
        config.max_message_size = Some(Self::MAX_FRAME_BYTES);
        config.max_frame_size = Some(Self::MAX_FRAME_BYTES);

        let socket = tokio_tungstenite::accept_async_with_config(tls, Some(config))
            .await
            .map_err(|source| WebSocketListenerError::Upgrade { peer, source })?;

        curia::info!("WebSocket session authenticated", { "peer": peer.to_string(), "device": device, "identity": identity.to_string() });

        let (mut sink, mut source) = socket.split();
        let (inbound_tx, inbound_rx) = mpsc::channel::<Bytes>(Self::READ_QUEUE_DEPTH);
        let (outbound_tx, mut outbound_rx) = mpsc::channel::<Bytes>(Self::WRITE_HANDOFF_DEPTH);

        let writer = tokio::spawn(async move {
            let mut queue: VecDeque<Bytes> = VecDeque::new();
            let mut dropped: u64 = 0;

            loop {
                if queue.is_empty() {
                    match outbound_rx.recv().await {
                        Some(payload) => queue.push_back(payload),
                        None => break,
                    }
                }

                // Take everything already waiting before writing, so the discard below
                // sees the true depth rather than one frame at a time.
                while let Ok(payload) = outbound_rx.try_recv() {
                    queue.push_back(payload);
                    if queue.len() > Self::WRITE_QUEUE_DEPTH {
                        queue.pop_front();
                        dropped += 1;
                    }
                }

                let payload = match queue.pop_front() {
                    Some(payload) => payload,
                    None => continue,
                };

                if sink.send(Message::Binary(payload)).await.is_err() {
                    break;
                }
            }

            if dropped > 0 {
                curia::debug!("websocket write queue shed frames", { "device": device, "dropped": dropped });
            }

            let _ = sink.close().await;
        });

        let reader = tokio::spawn(async move {
            while let Some(message) = source.next().await {
                let payload = match message {
                    Ok(Message::Binary(payload)) => payload,
                    // A packet is a binary frame. Text is not something this protocol
                    // produces, so it is ignored rather than guessed at.
                    Ok(Message::Close(_)) => break,
                    Ok(_) => continue,
                    Err(e) => {
                        curia::debug!(format!("websocket read ended: {e}"), { "device": device });
                        break;
                    }
                };

                if inbound_tx.send(payload).await.is_err() {
                    break;
                }
            }
        });

        let link = WsLink::new(device, inbound_rx, outbound_tx);
        spawner
            .run(SessionLink::WebSocket(link), device, Some(identity), fingerprint)
            .await;

        reader.abort();
        writer.abort();

        Ok(())
    }

    /// The verified leaf certificate this session presented.
    ///
    /// Classification and authorization happen in `SessionAuthorizationService`, which the
    /// QUIC handshake also uses — a peer CN is refused there rather than here, so the two
    /// transports cannot drift into admitting different populations.
    fn presented_leaf(tls: &TlsStream<TcpStream>) -> Option<Vec<u8>> {
        tls.get_ref()
            .1
            .peer_certificates()
            .and_then(|chain| chain.first())
            .map(|cert| cert.as_ref().to_vec())
    }

    fn client_verifier(
        ca_path: &str,
    ) -> Result<Arc<dyn rustls::server::danger::ClientCertVerifier>, WebSocketListenerError> {
        let mut roots = RootCertStore::empty();
        for certificate in Self::load_certificates(ca_path)? {
            roots
                .add(certificate)
                .map_err(|source| WebSocketListenerError::TrustRoot {
                    path: ca_path.to_string(),
                    source,
                })?;
        }

        WebPkiClientVerifier::builder(Arc::new(roots))
            .build()
            .map_err(|source| WebSocketListenerError::ClientVerifier { source })
    }

    fn load_certificates(
        path: &str,
    ) -> Result<Vec<CertificateDer<'static>>, WebSocketListenerError> {
        let pem = std::fs::read(path).map_err(|source| WebSocketListenerError::ReadFile {
            path: path.to_string(),
            source,
        })?;
        rustls_pemfile::certs(&mut pem.as_slice())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| WebSocketListenerError::ParseCertificates {
                path: path.to_string(),
                source,
            })
    }

    fn load_key(path: &str) -> Result<PrivateKeyDer<'static>, WebSocketListenerError> {
        let pem = std::fs::read(path).map_err(|source| WebSocketListenerError::ReadFile {
            path: path.to_string(),
            source,
        })?;
        rustls_pemfile::private_key(&mut pem.as_slice())
            .map_err(|source| WebSocketListenerError::ParseCertificates {
                path: path.to_string(),
                source,
            })?
            .ok_or_else(|| WebSocketListenerError::MissingPrivateKey {
                path: path.to_string(),
            })
    }
}
