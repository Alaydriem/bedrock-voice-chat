use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use hickory_proto::op::Message;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::task::JoinHandle;

use super::{DnsAnswerBuilder, DnsScript};

/// A DNS server on loopback, UDP and TCP on one port, that answers from a script.
pub struct DnsResponder {
    addr: SocketAddr,
    queries: Arc<AtomicUsize>,
    tasks: Vec<JoinHandle<()>>,
}

impl DnsResponder {
    pub async fn start(script: DnsScript) -> Self {
        let (udp, tcp) = Self::bind_pair().await;
        let addr = udp.local_addr().expect("a bound address");
        let queries = Arc::new(AtomicUsize::new(0));

        let udp_task = tokio::spawn(Self::serve_udp(udp, script.clone(), queries.clone()));
        let tcp_task = tokio::spawn(Self::serve_tcp(tcp, script, queries.clone()));

        Self {
            addr,
            queries,
            tasks: vec![udp_task, tcp_task],
        }
    }

    // The `host:port` form a resolver config takes.
    pub fn nameserver(&self) -> String {
        self.addr.to_string()
    }

    pub fn queries(&self) -> usize {
        self.queries.load(Ordering::SeqCst)
    }

    async fn bind_pair() -> (UdpSocket, TcpListener) {
        for _ in 0..50 {
            let udp = UdpSocket::bind("127.0.0.1:0").await.expect("a UDP socket");
            let port = udp.local_addr().expect("a bound address").port();
            if let Ok(tcp) = TcpListener::bind(("127.0.0.1", port)).await {
                return (udp, tcp);
            }
        }
        panic!("no port was free for both UDP and TCP");
    }

    async fn serve_udp(socket: UdpSocket, script: DnsScript, queries: Arc<AtomicUsize>) {
        let mut buffer = [0u8; 2048];
        loop {
            let Ok((length, peer)) = socket.recv_from(&mut buffer).await else {
                return;
            };
            queries.fetch_add(1, Ordering::SeqCst);
            let Ok(request) = Message::from_vec(&buffer[..length]) else {
                continue;
            };
            if let Some(reply) = DnsAnswerBuilder::response(&request, &script, false) {
                let _ = socket.send_to(&reply, peer).await;
            }
        }
    }

    async fn serve_tcp(listener: TcpListener, script: DnsScript, queries: Arc<AtomicUsize>) {
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
        let mut prefix = [0u8; 2];
        if stream.read_exact(&mut prefix).await.is_err() {
            return;
        }
        let mut body = vec![0u8; u16::from_be_bytes(prefix) as usize];
        if stream.read_exact(&mut body).await.is_err() {
            return;
        }
        queries.fetch_add(1, Ordering::SeqCst);
        let Ok(request) = Message::from_vec(&body) else {
            return;
        };
        if let Some(reply) = DnsAnswerBuilder::response(&request, &script, true) {
            let mut framed = (reply.len() as u16).to_be_bytes().to_vec();
            framed.extend_from_slice(&reply);
            let _ = stream.write_all(&framed).await;
        }
    }
}

impl Drop for DnsResponder {
    fn drop(&mut self) {
        for task in &self.tasks {
            task.abort();
        }
    }
}
