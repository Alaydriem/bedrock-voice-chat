use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use hickory_proto::op::Message;
use tokio::net::UdpSocket;
use tokio::task::JoinHandle;

use super::{DnsAnswerBuilder, DnsScript};

/// A UDP DNS server on loopback that answers from a script, which a test can change.
pub struct DnsResponder {
    addr: SocketAddr,
    queries: Arc<AtomicUsize>,
    script: Arc<Mutex<DnsScript>>,
    task: JoinHandle<()>,
}

impl DnsResponder {
    pub async fn start(script: DnsScript) -> Self {
        let socket = UdpSocket::bind("127.0.0.1:0").await.expect("a UDP socket");
        let addr = socket.local_addr().expect("a bound address");
        let queries = Arc::new(AtomicUsize::new(0));
        let script = Arc::new(Mutex::new(script));
        let task = tokio::spawn(Self::serve(socket, script.clone(), queries.clone()));

        Self {
            addr,
            queries,
            script,
            task,
        }
    }

    // The `host:port` form a resolver config takes.
    pub fn nameserver(&self) -> String {
        self.addr.to_string()
    }

    pub fn queries(&self) -> usize {
        self.queries.load(Ordering::SeqCst)
    }

    pub fn answer_with(&self, script: DnsScript) {
        *self.script.lock().expect("an unpoisoned script") = script;
    }

    async fn serve(socket: UdpSocket, script: Arc<Mutex<DnsScript>>, queries: Arc<AtomicUsize>) {
        let mut buffer = [0u8; 2048];
        loop {
            let Ok((length, peer)) = socket.recv_from(&mut buffer).await else {
                return;
            };
            queries.fetch_add(1, Ordering::SeqCst);
            let Ok(request) = Message::from_vec(&buffer[..length]) else {
                continue;
            };
            let current = script.lock().expect("an unpoisoned script").clone();
            if let Some(reply) = DnsAnswerBuilder::response(&request, &current) {
                let _ = socket.send_to(&reply, peer).await;
            }
        }
    }
}

impl Drop for DnsResponder {
    fn drop(&mut self) {
        self.task.abort();
    }
}
