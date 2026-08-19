use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use serde::Serialize;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;

/// THROWAWAY. Measures whether this platform's webview may open a loopback socket at all.
///
/// Delete this module, `commands::spike`, `js/app/spike` and the About pane card with it once
/// the answer is recorded. Nothing else may depend on it.
///
/// The counters are the point. A webview refuses a connection with an untyped `error` event and
/// close code 1006 whatever the cause, so the page alone cannot separate a platform policy that
/// dropped the connection before it left the process from a handshake that failed after it
/// arrived. Counting accepts on this side answers that without a console, which iPadOS does not
/// readily give up.
#[derive(Default)]
pub struct LoopbackProbe {
    tcp_accepted: AtomicU32,
    ws_upgraded: AtomicU32,
    http_served: AtomicU32,
    ports: std::sync::Mutex<Option<ProbePorts>>,
}

/// Where the page should aim. Two listeners, because the interesting failure is asymmetric:
/// cleartext policy refuses both, and a WebSocket-specific rule refuses only the first.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct ProbePorts {
    pub ws: u16,
    pub http: u16,
}

/// What the two sides each observed, for the page to render side by side.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct ProbeStats {
    pub tcp_accepted: u32,
    pub ws_upgraded: u32,
    pub http_served: u32,
}

impl LoopbackProbe {
    /// How often a connected socket is sent a frame, and how many it is sent.
    ///
    /// Fast and finite: the page waits on the first few, and a probe listener must not outlive
    /// the panel that opened it.
    const PUSH_INTERVAL: std::time::Duration = std::time::Duration::from_millis(250);
    const PUSH_COUNT: u32 = 40;

    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Bind both listeners on loopback, or report why not.
    ///
    /// Idempotent: a second press reuses the ports rather than leaking a listener.
    pub async fn start(self: &Arc<Self>) -> Result<ProbePorts, anyhow::Error> {
        if let Some(ports) = self.ports.lock().ok().and_then(|slot| *slot) {
            return Ok(ports);
        }

        let ws_listener = TcpListener::bind("127.0.0.1:0").await?;
        let http_listener = TcpListener::bind("127.0.0.1:0").await?;
        let ports = ProbePorts {
            ws: ws_listener.local_addr()?.port(),
            http: http_listener.local_addr()?.port(),
        };

        self.spawn_ws_loop(ws_listener);
        self.spawn_http_loop(http_listener);

        if let Ok(mut slot) = self.ports.lock() {
            *slot = Some(ports);
        }
        log::info!(
            "loopback probe listening: ws=127.0.0.1:{} http=127.0.0.1:{}",
            ports.ws,
            ports.http
        );
        Ok(ports)
    }

    pub fn stats(&self) -> ProbeStats {
        ProbeStats {
            tcp_accepted: self.tcp_accepted.load(Ordering::Relaxed),
            ws_upgraded: self.ws_upgraded.load(Ordering::Relaxed),
            http_served: self.http_served.load(Ordering::Relaxed),
        }
    }

    fn spawn_ws_loop(self: &Arc<Self>, listener: TcpListener) {
        let probe = self.clone();
        tokio::spawn(async move {
            while let Ok((stream, peer)) = listener.accept().await {
                probe.tcp_accepted.fetch_add(1, Ordering::Relaxed);
                log::info!("loopback probe: ws socket accepted from {}", peer);
                let probe = probe.clone();
                tokio::spawn(async move {
                    if let Err(e) = probe.serve_ws(stream).await {
                        log::warn!("loopback probe: ws connection ended: {}", e);
                    }
                });
            }
        });
    }

    async fn serve_ws(&self, stream: tokio::net::TcpStream) -> Result<(), anyhow::Error> {
        use futures_util::SinkExt;

        let mut ws = tokio_tungstenite::accept_async(stream).await?;
        self.ws_upgraded.fetch_add(1, Ordering::Relaxed);
        log::info!("loopback probe: ws handshake completed");

        for seq in 0..Self::PUSH_COUNT {
            tokio::time::sleep(Self::PUSH_INTERVAL).await;
            let frame = format!("{{\"type\":\"probe\",\"seq\":{}}}", seq);
            ws.send(tokio_tungstenite::tungstenite::Message::Text(frame.into()))
                .await?;
        }

        ws.close(None).await?;
        Ok(())
    }

    /// A fixed 200, whatever is asked for. Its only job is to be reachable over cleartext, so a
    /// refused `fetch` and a refused WebSocket can be told apart.
    fn spawn_http_loop(self: &Arc<Self>, listener: TcpListener) {
        let probe = self.clone();
        tokio::spawn(async move {
            while let Ok((mut stream, peer)) = listener.accept().await {
                probe.tcp_accepted.fetch_add(1, Ordering::Relaxed);
                probe.http_served.fetch_add(1, Ordering::Relaxed);
                log::info!("loopback probe: http socket accepted from {}", peer);
                tokio::spawn(async move {
                    let body = "probe-ok";
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = stream.write_all(response.as_bytes()).await;
                    let _ = stream.shutdown().await;
                });
            }
        });
    }
}
