use std::sync::Arc;

use common::rustls::{BvcClientTls, VerifierMode};
use common::structs::websocket::PositionSocketEvent;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::Connector;

/// The dashboard's position feed socket.
///
/// Opened in Rust because the webview validates certificates against the operating system's
/// roots, which do not include the play roots. Frames reach the webview as
/// `PositionSocketEvent`s. The ticket that authenticates the socket travels as a
/// subprotocol, exactly as it did from the webview.
pub(crate) struct PositionSocket;

impl PositionSocket {
    /// Dials `url` offering `protocols`, then relays frames to `events` until the server
    /// closes, the link fails, or `stop` fires.
    pub(crate) async fn run(
        url: String,
        protocols: Vec<String>,
        events: mpsc::Sender<PositionSocketEvent>,
        mut stop: oneshot::Receiver<()>,
    ) {
        let connected = tokio::select! {
            result = Self::connect(&url, &protocols) => result,
            _ = &mut stop => return,
        };
        let socket = match connected {
            Ok(socket) => socket,
            Err(error) => {
                let _ = events
                    .send(PositionSocketEvent::Error {
                        reason: error.to_string(),
                    })
                    .await;
                return;
            }
        };

        if events.send(PositionSocketEvent::Open).await.is_err() {
            return;
        }

        let (mut sink, mut source) = socket.split();
        loop {
            tokio::select! {
                _ = &mut stop => {
                    let _ = sink.close().await;
                    return;
                }
                message = source.next() => match message {
                    Some(Ok(Message::Text(text))) => {
                        let data = text.to_string();
                        if events.send(PositionSocketEvent::Message { data }).await.is_err() {
                            return;
                        }
                    }
                    Some(Ok(Message::Ping(payload))) => {
                        let _ = sink.send(Message::Pong(payload)).await;
                    }
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                    Some(Ok(_)) => {}
                },
            }
        }

        let _ = events.send(PositionSocketEvent::Closed).await;
    }

    async fn connect(
        url: &str,
        protocols: &[String],
    ) -> Result<
        tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        anyhow::Error,
    > {
        let tls = BvcClientTls::config(None, None, &[b"http/1.1"], VerifierMode::for_build())?;
        let mut request = url.into_client_request()?;
        request.headers_mut().insert(
            "Sec-WebSocket-Protocol",
            HeaderValue::from_str(&protocols.join(", "))?,
        );

        let (socket, _) = tokio_tungstenite::connect_async_tls_with_config(
            request,
            None,
            false,
            Some(Connector::Rustls(Arc::new(tls))),
        )
        .await?;

        Ok(socket)
    }
}
