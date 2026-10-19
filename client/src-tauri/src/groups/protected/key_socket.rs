use std::sync::Arc;

use common::structs::group_key::GroupKeyFrame;
use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{Connector, WebSocketStream};

use super::KeySocketEvent;
use crate::network::MtlsClientConfig;

/// A protected group's key socket to the server.
///
/// It authenticates with the player's certificate and offers ALPN `http/1.1`, so the server's
/// demultiplexer hands it to the API rather than the voice listener on the same port.
pub struct KeySocket;

impl KeySocket {
    const QUEUE_DEPTH: usize = 256;

    pub async fn connect(
        server_url: &str,
        ca_cert: &str,
        pem: &str,
        group_id: &str,
    ) -> anyhow::Result<(mpsc::Sender<GroupKeyFrame>, mpsc::Receiver<KeySocketEvent>)> {
        let authority = server_url
            .strip_prefix("https://")
            .or_else(|| server_url.strip_prefix("http://"))
            .unwrap_or(server_url)
            .trim_end_matches('/');
        let request =
            format!("wss://{authority}/api/websocket/group/{group_id}").into_client_request()?;
        // The player's PEM carries both the certificate chain and the key.
        let config = MtlsClientConfig::build(ca_cert, pem, pem, &[b"http/1.1"])?;
        let (socket, _) = tokio_tungstenite::connect_async_tls_with_config(
            request,
            None,
            false,
            Some(Connector::Rustls(Arc::new(config))),
        )
        .await?;
        Ok(Self::spawn(socket))
    }

    /// Pumps frames between a connected socket and two queues until either side ends.
    pub fn spawn<S>(
        socket: WebSocketStream<S>,
    ) -> (mpsc::Sender<GroupKeyFrame>, mpsc::Receiver<KeySocketEvent>)
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        let (outbound_tx, mut outbound_rx) = mpsc::channel::<GroupKeyFrame>(Self::QUEUE_DEPTH);
        let (event_tx, event_rx) = mpsc::channel::<KeySocketEvent>(Self::QUEUE_DEPTH);
        let (mut sink, mut source) = socket.split();

        tokio::spawn(async move {
            while let Some(frame) = outbound_rx.recv().await {
                let Ok(bytes) = frame.to_bytes() else {
                    continue;
                };
                if sink.send(Message::Binary(bytes.into())).await.is_err() {
                    break;
                }
            }
            let _ = sink.close().await;
        });

        tokio::spawn(async move {
            let mut reason = None;
            while let Some(message) = source.next().await {
                match message {
                    Ok(Message::Binary(bytes)) => match GroupKeyFrame::from_bytes(&bytes) {
                        Ok(frame) => {
                            if event_tx.send(KeySocketEvent::Frame(frame)).await.is_err() {
                                return;
                            }
                        }
                        Err(_) => log::debug!("protected group: undecodable key socket frame"),
                    },
                    Ok(Message::Close(frame)) => {
                        reason = frame
                            .map(|f| f.reason.to_string())
                            .filter(|r| !r.is_empty());
                        break;
                    }
                    Ok(_) => continue,
                    Err(_) => break,
                }
            }
            let _ = event_tx.send(KeySocketEvent::Closed(reason)).await;
        });

        (outbound_tx, event_rx)
    }
}
