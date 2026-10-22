use super::{MtlsClientConfig, RecvFailure, VoiceLinkError};
use bytes::Bytes;
use common::structs::network::VoiceProtocol;
use futures_util::{SinkExt, StreamExt};
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
    ) -> Result<Self, VoiceLinkError> {
        let config = MtlsClientConfig::build(ca_cert, cert, key, &[VoiceProtocol::ALPN])?;
        let request = url
            .into_client_request()
            .map_err(|source| VoiceLinkError::InvalidUrl {
                url: url.to_string(),
                source,
            })?;

        let (socket, _) = tokio_tungstenite::connect_async_tls_with_config(
            request,
            None,
            false,
            Some(Connector::Rustls(Arc::new(config))),
        )
        .await
        .map_err(|source| VoiceLinkError::Connect {
            url: url.to_string(),
            source,
        })?;

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

    pub(crate) fn send(&self, payload: Bytes) -> Result<(), VoiceLinkError> {
        match self.outbound.try_send(payload) {
            Ok(()) => Ok(()),
            // The write pump bounds its own queue and sheds the oldest frame. Reaching
            // here means even the handoff is saturated, which is the same shed-and-carry-on
            // case a full QUIC send queue is.
            Err(mpsc::error::TrySendError::Full(_)) => Err(VoiceLinkError::SendQueueFull),
            Err(mpsc::error::TrySendError::Closed(_)) => Err(VoiceLinkError::Closed),
        }
    }

}
