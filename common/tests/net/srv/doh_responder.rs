use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use hickory_proto::op::Message;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;

use super::{DnsAnswerBuilder, DnsScript};

/// A plain-HTTP RFC 8484 responder on loopback. A `Silent` script answers HTTP 500.
pub struct DohResponder {
    addr: SocketAddr,
    queries: Arc<AtomicUsize>,
    task: JoinHandle<()>,
}

impl DohResponder {
    pub async fn start(script: DnsScript) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("a TCP listener");
        let addr = listener.local_addr().expect("a bound address");
        let queries = Arc::new(AtomicUsize::new(0));
        let task = tokio::spawn(Self::serve(listener, script, queries.clone()));

        Self {
            addr,
            queries,
            task,
        }
    }

    pub fn url(&self) -> String {
        format!("http://{}/dns-query", self.addr)
    }

    pub fn queries(&self) -> usize {
        self.queries.load(Ordering::SeqCst)
    }

    async fn serve(listener: TcpListener, script: DnsScript, queries: Arc<AtomicUsize>) {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                return;
            };
            tokio::spawn(Self::serve_connection(
                stream,
                script.clone(),
                queries.clone(),
            ));
        }
    }

    async fn serve_connection(mut stream: TcpStream, script: DnsScript, queries: Arc<AtomicUsize>) {
        let Some(body) = Self::read_body(&mut stream).await else {
            return;
        };
        queries.fetch_add(1, Ordering::SeqCst);

        if matches!(script, DnsScript::Hang) {
            std::future::pending::<()>().await;
        }

        let reply = Message::from_vec(&body)
            .ok()
            .and_then(|request| DnsAnswerBuilder::response(&request, &script, true));

        let response = match reply {
            Some(reply) => {
                let mut response = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/dns-message\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                    reply.len()
                )
                .into_bytes();
                response.extend_from_slice(&reply);
                response
            }
            None => b"HTTP/1.1 500 Internal Server Error\r\ncontent-length: 0\r\nconnection: close\r\n\r\n"
                .to_vec(),
        };
        let _ = stream.write_all(&response).await;
        let _ = stream.shutdown().await;
    }

    async fn read_body(stream: &mut TcpStream) -> Option<Vec<u8>> {
        let mut received = Vec::new();
        let mut chunk = [0u8; 1024];
        let header_end = loop {
            let read = stream.read(&mut chunk).await.ok()?;
            if read == 0 {
                return None;
            }
            received.extend_from_slice(&chunk[..read]);
            if let Some(position) = received.windows(4).position(|window| window == b"\r\n\r\n") {
                break position + 4;
            }
        };

        let headers = String::from_utf8_lossy(&received[..header_end]).to_ascii_lowercase();
        let length: usize = headers
            .lines()
            .find_map(|line| line.strip_prefix("content-length:"))
            .and_then(|value| value.trim().parse().ok())?;

        while received.len() < header_end + length {
            let read = stream.read(&mut chunk).await.ok()?;
            if read == 0 {
                return None;
            }
            received.extend_from_slice(&chunk[..read]);
        }
        Some(received[header_end..header_end + length].to_vec())
    }
}

impl Drop for DohResponder {
    fn drop(&mut self) {
        self.task.abort();
    }
}
