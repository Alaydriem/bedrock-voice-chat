use super::RecvFailure;
use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::{ClientConfig, RootCertStore};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use tokio_tungstenite::Connector;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

/// A voice session carried over TLS WebSocket instead of QUIC datagrams.
///
/// Exists for the networks QUIC never leaves: carriers that drop UDP outright, and
/// middleboxes that permit the handshake and then degrade the session. TCP costs latency
/// under loss, which is the trade — a slower link the player can hear beats a faster one
/// that carries nothing.
#[derive(Clone)]
pub(crate) struct WsLink {
    // Only the input stream reads. The mutex gives this the shared-ownership shape the
    // QUIC link already has rather than arbitrating between contenders.
    inbound: Arc<Mutex<mpsc::Receiver<Bytes>>>,
    outbound: mpsc::Sender<Bytes>,
}

impl WsLink {
    /// The protocol that tells the server's demultiplexer to route this to the voice
    /// listener rather than to the API. A server too old to have one offers no such
    /// protocol and the handshake fails fast, which is why no capability negotiation is
    /// needed before dialling.
    const ALPN: &'static [u8] = b"bvc-ws/1";

    // One frame is one packet, so these are measured in packets exactly like the QUIC
    // datagram capacities.
    const READ_QUEUE_DEPTH: usize = 256;
    const WRITE_HANDOFF_DEPTH: usize = 64;

    // Under backpressure the write pump discards the OLDEST frame: late audio has already
    // missed its moment, and keeping it ahead of fresher audio costs latency for the rest
    // of the session.
    const WRITE_QUEUE_DEPTH: usize = 256;

    pub(crate) async fn connect(
        url: &str,
        ca_cert: &str,
        cert: &str,
        key: &str,
    ) -> Result<Self, anyhow::Error> {
        let config = Self::tls_config(ca_cert, cert, key)?;
        let request = url
            .into_client_request()
            .map_err(|e| anyhow::anyhow!("building the WebSocket request for {url}: {e}"))?;

        let (socket, _) = tokio_tungstenite::connect_async_tls_with_config(
            request,
            None,
            false,
            Some(Connector::Rustls(Arc::new(config))),
        )
        .await
        .map_err(|e| anyhow::anyhow!("connecting the WebSocket transport to {url}: {e}"))?;

        let (mut sink, mut source) = socket.split();
        let (inbound_tx, inbound_rx) = mpsc::channel::<Bytes>(Self::READ_QUEUE_DEPTH);
        let (outbound_tx, mut outbound_rx) = mpsc::channel::<Bytes>(Self::WRITE_HANDOFF_DEPTH);

        tokio::spawn(async move {
            let mut queue: VecDeque<Bytes> = VecDeque::new();

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
                    }
                }

                let Some(payload) = queue.pop_front() else {
                    continue;
                };

                if sink.send(Message::Binary(payload)).await.is_err() {
                    break;
                }
            }

            let _ = sink.close().await;
        });

        tokio::spawn(async move {
            while let Some(message) = source.next().await {
                let payload = match message {
                    Ok(Message::Binary(payload)) => payload,
                    Ok(Message::Close(_)) => break,
                    Ok(_) => continue,
                    Err(_) => break,
                };

                if inbound_tx.send(payload).await.is_err() {
                    break;
                }
            }
        });

        Ok(Self {
            inbound: Arc::new(Mutex::new(inbound_rx)),
            outbound: outbound_tx,
        })
    }

    pub(crate) async fn recv(&self) -> Result<Bytes, RecvFailure> {
        self.inbound
            .lock()
            .await
            .recv()
            .await
            .ok_or_else(|| RecvFailure::Closed("websocket closed".to_string()))
    }

    pub(crate) fn send(&self, payload: Bytes) -> Result<(), anyhow::Error> {
        match self.outbound.try_send(payload) {
            Ok(()) => Ok(()),
            // The write pump bounds its own queue and sheds the oldest frame. Reaching
            // here means even the handoff is saturated, which is the same shed-and-carry-on
            // case a full QUIC send queue is.
            Err(mpsc::error::TrySendError::Full(_)) => {
                Err(anyhow::anyhow!("websocket send queue full"))
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                Err(anyhow::anyhow!("websocket write pump stopped"))
            }
        }
    }

    /// Trusts the public roots plus the server's own CA, and presents this player's
    /// certificate.
    ///
    /// The additional-root shape mirrors the HTTP client (`api/client.rs`): a hosted
    /// server presents a publicly-signed certificate, a self-hosted one presents a
    /// CA-signed certificate, and both must work without an operator choosing.
    fn tls_config(ca_cert: &str, cert: &str, key: &str) -> Result<ClientConfig, anyhow::Error> {
        let mut roots = RootCertStore::from_iter(
            webpki_roots::TLS_SERVER_ROOTS
                .iter()
                .cloned(),
        );
        for certificate in Self::parse_certificates(ca_cert)? {
            roots
                .add(certificate)
                .map_err(|e| anyhow::anyhow!("adding the server CA to the trust roots: {e}"))?;
        }

        // Named explicitly rather than taken from the process default: whether one is
        // installed depends on which other component initialised rustls first, and
        // `ClientConfig::builder` panics rather than erroring when none is.
        let builder = ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::aws_lc_rs::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .map_err(|e| anyhow::anyhow!("selecting TLS protocol versions: {e}"))?
        .with_root_certificates(roots);

        let mut config = builder
            .with_client_auth_cert(Self::parse_certificates(cert)?, Self::parse_key(key)?)
            .map_err(|e| anyhow::anyhow!("presenting the client certificate: {e}"))?;

        config.alpn_protocols = vec![Self::ALPN.to_vec()];

        // Matches the HTTP client, which sets `danger_accept_invalid_certs` under the same
        // condition. Development and the end-to-end harness both run servers whose
        // certificates no root will vouch for; a release build validates normally.
        #[cfg(debug_assertions)]
        {
            config
                .dangerous()
                .set_certificate_verifier(Arc::new(super::PermissiveServerVerifier::new()));
        }

        Ok(config)
    }

    fn parse_certificates(pem: &str) -> Result<Vec<CertificateDer<'static>>, anyhow::Error> {
        rustls_pemfile::certs(&mut pem.as_bytes())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| anyhow::anyhow!("parsing certificates: {e}"))
    }

    fn parse_key(pem: &str) -> Result<PrivateKeyDer<'static>, anyhow::Error> {
        rustls_pemfile::private_key(&mut pem.as_bytes())
            .map_err(|e| anyhow::anyhow!("parsing the client private key: {e}"))?
            .ok_or_else(|| anyhow::anyhow!("no private key found in the client identity"))
    }
}
