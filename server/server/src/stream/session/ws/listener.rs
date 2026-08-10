use super::WsLink;
use crate::demux::AlpnDemux;
use crate::stream::quic::{CertificateCommonName, ConnectionClassifier, ConnectionKind};
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
/// client offers the `bvc-ws/1` protocol.
pub struct WebSocketListener {
    listener: TcpListener,
    acceptor: TlsAcceptor,
    spawner: Arc<SessionSpawner>,
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
    pub async fn bind(
        certificate_path: &str,
        key_path: &str,
        ca_path: &str,
        spawner: Arc<SessionSpawner>,
    ) -> Result<(Self, SocketAddr), anyhow::Error> {
        // Named explicitly rather than taken from the process default: whether one is
        // installed depends on which other component initialised rustls first, and
        // `ServerConfig::builder` panics rather than erroring when none is.
        let mut config = ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::aws_lc_rs::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .map_err(|e| anyhow::anyhow!("selecting TLS protocol versions: {e}"))?
        .with_client_cert_verifier(Self::client_verifier(ca_path)?)
            .with_single_cert(
                Self::load_certificates(certificate_path)?,
                Self::load_key(key_path)?,
            )
            .map_err(|e| anyhow::anyhow!("building the WebSocket TLS config: {e}"))?;

        config.alpn_protocols = vec![AlpnDemux::WEBSOCKET_ALPN.to_vec()];

        let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))
            .await
            .map_err(|e| anyhow::anyhow!("binding the WebSocket listener: {e}"))?;
        let addr = listener.local_addr()?;

        Ok((
            Self {
                listener,
                acceptor: TlsAcceptor::from(Arc::new(config)),
                spawner,
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

    pub async fn start(self) -> Result<(), anyhow::Error> {
        tracing::info!(bind = %self.listener.local_addr()?, "WebSocket voice listener started");

        loop {
            let (stream, peer) = match self.listener.accept().await {
                Ok(accepted) => accepted,
                Err(e) => {
                    tracing::warn!("accepting on the WebSocket listener: {e}");
                    continue;
                }
            };

            let acceptor = self.acceptor.clone();
            let spawner = self.spawner.clone();
            let device = self.devices.next();
            let metrics = self.metrics.clone();

            tokio::spawn(async move {
                match Self::serve(stream, peer, acceptor, spawner, device).await {
                    Ok(()) => {}
                    Err(e) => {
                        // Everything that fails before `serve` reaches the spawner is a
                        // refusal: a TLS handshake that did not complete, a certificate
                        // naming no player, an upgrade that never finished.
                        if let Some(metrics) = metrics {
                            metrics.record_websocket_rejection();
                        }
                        tracing::debug!(%peer, device, "websocket session ended: {e}");
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
        device: u64,
    ) -> Result<(), anyhow::Error> {
        let tls = acceptor
            .accept(stream)
            .await
            .map_err(|e| anyhow::anyhow!("websocket TLS handshake from {peer}: {e}"))?;

        let identity = Self::authenticated_identity(&tls).ok_or_else(|| {
            anyhow::anyhow!("refusing {peer}: no usable player identity in the client certificate")
        })?;

        let mut config = tokio_tungstenite::tungstenite::protocol::WebSocketConfig::default();
        config.max_message_size = Some(Self::MAX_FRAME_BYTES);
        config.max_frame_size = Some(Self::MAX_FRAME_BYTES);

        let socket = tokio_tungstenite::accept_async_with_config(tls, Some(config))
            .await
            .map_err(|e| anyhow::anyhow!("websocket upgrade from {peer}: {e}"))?;

        tracing::info!(%peer, device, %identity, "WebSocket session authenticated");

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
                tracing::debug!(device, dropped, "websocket write queue shed frames");
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
                        tracing::debug!(device, "websocket read ended: {e}");
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
            .run(SessionLink::WebSocket(link), device, Some(identity), None, None)
            .await;

        reader.abort();
        writer.abort();

        Ok(())
    }

    /// The player identity this session proved, or `None` if it proved none.
    ///
    /// A peer CN is refused rather than accepted: peer links carry another server's
    /// speakers over a relay path that is server-to-server QUIC by nature, and nothing
    /// downstream of here would know the difference.
    fn authenticated_identity(tls: &TlsStream<TcpStream>) -> Option<String> {
        let common_name = tls
            .get_ref()
            .1
            .peer_certificates()
            .and_then(|chain| chain.first())
            .and_then(|cert| CertificateCommonName::from_der(cert))?;

        match ConnectionClassifier::classify(&common_name) {
            ConnectionKind::Player { game, name } => Some(game.membership_key(&name)),
            ConnectionKind::Peer { .. } => {
                tracing::warn!(
                    identity = %common_name,
                    "Refusing WebSocket session: peer links are QUIC only"
                );
                None
            }
            ConnectionKind::Rejected { identity } => {
                tracing::warn!(
                    %identity,
                    "Refusing WebSocket session: certificate identity is not a valid player CN"
                );
                None
            }
        }
    }

    fn client_verifier(
        ca_path: &str,
    ) -> Result<Arc<dyn rustls::server::danger::ClientCertVerifier>, anyhow::Error> {
        let mut roots = RootCertStore::empty();
        for certificate in Self::load_certificates(ca_path)? {
            roots
                .add(certificate)
                .map_err(|e| anyhow::anyhow!("adding {ca_path} to the client trust roots: {e}"))?;
        }

        WebPkiClientVerifier::builder(Arc::new(roots))
            .build()
            .map_err(|e| anyhow::anyhow!("building the client certificate verifier: {e}"))
    }

    fn load_certificates(path: &str) -> Result<Vec<CertificateDer<'static>>, anyhow::Error> {
        let pem = std::fs::read(path).map_err(|e| anyhow::anyhow!("reading {path}: {e}"))?;
        rustls_pemfile::certs(&mut pem.as_slice())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| anyhow::anyhow!("parsing certificates in {path}: {e}"))
    }

    fn load_key(path: &str) -> Result<PrivateKeyDer<'static>, anyhow::Error> {
        let pem = std::fs::read(path).map_err(|e| anyhow::anyhow!("reading {path}: {e}"))?;
        rustls_pemfile::private_key(&mut pem.as_slice())
            .map_err(|e| anyhow::anyhow!("parsing the private key in {path}: {e}"))?
            .ok_or_else(|| anyhow::anyhow!("no private key found in {path}"))
    }
}
