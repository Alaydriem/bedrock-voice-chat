use std::time::Duration;

use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

use super::HeadVerdict;
use crate::websocket::ListenerKind;

/// Answers a plain HTTP GET on the operator listener, so OBS can load the overlay from the
/// same origin it will open its WebSocket against.
///
/// Same origin is the whole reason this lives on the WebSocket port rather than one of its own:
/// Chromium gates a request from a public origin to loopback behind a permission prompt that an
/// OBS browser source has no way to answer, and a page served from loopback is never subject to
/// it.
pub struct OverlayHttpResponder;

impl OverlayHttpResponder {
    /// The page itself, compiled in so there is no file to lose or to package.
    const PAGE: &'static str = include_str!("../../../resources/overlay.html");

    const PATH: &'static str = "/overlay";

    /// How much of the request head is read before giving up on classifying it.
    const PEEK_LIMIT: usize = 4096;

    /// How long to wait for a head to finish arriving.
    ///
    /// On loopback a browser sends the whole head in one segment, so this is only ever spent on
    /// a peer that is not going to say anything useful.
    const PEEK_TIMEOUT: Duration = Duration::from_millis(250);

    /// How long to wait before looking again when nothing new has arrived.
    const PEEK_RETRY: Duration = Duration::from_millis(5);

    /// Decide what this connection is, and answer it if it is ours.
    ///
    /// `Ok(false)` leaves the stream untouched and positioned at its first byte, which is what
    /// the handshake expects. Peeking rather than reading is what makes that possible.
    pub async fn answer(
        stream: &mut TcpStream,
        kind: ListenerKind,
        credential: &str,
    ) -> Result<bool, anyhow::Error> {
        // The internal listener has no operator credential and nothing dials it with a browser.
        if kind != ListenerKind::External {
            return Ok(false);
        }

        let Some((path, query)) = Self::peek_target(stream).await? else {
            return Ok(false);
        };

        if path.trim_end_matches('/') != Self::PATH {
            Self::respond(stream, 404, "text/plain; charset=utf-8", "Not found").await?;
            return Ok(true);
        }

        if !Self::authorized(&query, credential) {
            Self::respond(
                stream,
                401,
                "text/plain; charset=utf-8",
                "The overlay needs the WebSocket access key. Add ?key=<your key> to the URL.",
            )
            .await?;
            return Ok(true);
        }

        Self::respond(stream, 200, "text/html; charset=utf-8", Self::PAGE).await?;
        Ok(true)
    }

    /// The same rule `WebSocketRoute` applies to a push route: an empty configured key accepts,
    /// because a user who has not set one has not asked for authentication.
    fn authorized(query: &str, credential: &str) -> bool {
        if credential.is_empty() {
            return true;
        }
        query
            .split('&')
            .filter_map(|pair| pair.split_once('='))
            .any(|(name, value)| name == "key" && value == credential)
    }

    async fn peek_target(stream: &TcpStream) -> Result<Option<(String, String)>, anyhow::Error> {
        let deadline = tokio::time::Instant::now() + Self::PEEK_TIMEOUT;
        let mut buf = vec![0u8; Self::PEEK_LIMIT];

        loop {
            let peeked = tokio::time::timeout_at(deadline, stream.peek(&mut buf)).await;
            let read = match peeked {
                Ok(Ok(0)) => return Ok(None),
                Ok(Ok(read)) => read,
                // A closed socket, or a peer that has sent nothing in the window. The handshake
                // has its own timeouts and is the better place for both to end up.
                Ok(Err(_)) | Err(_) => return Ok(None),
            };

            match HeadVerdict::classify(&String::from_utf8_lossy(&buf[..read])) {
                HeadVerdict::Upgrade => return Ok(None),
                HeadVerdict::Http { path, query } => return Ok(Some((path, query))),
                HeadVerdict::Incomplete => {}
            }

            if read >= Self::PEEK_LIMIT {
                return Ok(None);
            }

            // `peek` returns the same buffered bytes immediately, so waiting on it again would
            // spin. Sleep instead, and let the deadline end it.
            if tokio::time::Instant::now() >= deadline {
                return Ok(None);
            }
            tokio::time::sleep(Self::PEEK_RETRY).await;
        }
    }

    async fn respond(
        stream: &mut TcpStream,
        status: u16,
        content_type: &str,
        body: &str,
    ) -> Result<(), anyhow::Error> {
        let reason = match status {
            200 => "OK",
            401 => "Unauthorized",
            _ => "Not Found",
        };
        let head = format!(
            "HTTP/1.1 {status} {reason}\r\n\
             Content-Type: {content_type}\r\n\
             Content-Length: {}\r\n\
             Cache-Control: no-store\r\n\
             Connection: close\r\n\r\n",
            body.len()
        );

        // The peek left every byte the client sent in the socket. Nothing here reads them; the
        // shutdown is what ends the exchange, and a browser that sent a keep-alive request is
        // told otherwise by the header.
        stream.write_all(head.as_bytes()).await?;
        stream.write_all(body.as_bytes()).await?;
        stream.flush().await?;
        let _ = stream.shutdown().await;
        Ok(())
    }
}
