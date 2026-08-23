use std::net::Ipv4Addr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use tokio::net::UdpSocket;

/// A UDP port that accepts datagrams and discards every one of them.
///
/// Models a network that blocks QUIC rather than one that loses some of it: a closed port
/// answers ICMP and fails the dial immediately, where a filter swallows the packets and
/// the client learns nothing until its handshake budget expires.
///
/// Not `LossyUdpRelay`, which asserts some traffic gets through.
pub struct UdpBlackhole {
    port: u16,
    swallowed: Arc<AtomicU64>,
    shutdown: Arc<AtomicBool>,
}

impl UdpBlackhole {
    /// Binds an ephemeral loopback port and swallows everything sent to it.
    pub async fn start() -> Self {
        let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .expect("bind the blackhole socket");
        let port = socket.local_addr().expect("blackhole local addr").port();

        let swallowed = Arc::new(AtomicU64::new(0));
        let shutdown = Arc::new(AtomicBool::new(false));

        let reader_swallowed = swallowed.clone();
        let reader_shutdown = shutdown.clone();
        tokio::spawn(async move {
            let mut buffer = vec![0u8; 2048];

            // Drained rather than left to fill: a full receive queue can make the host
            // answer ICMP, which would turn this into a closed port.
            while !reader_shutdown.load(Ordering::Relaxed) {
                match socket.recv_from(&mut buffer).await {
                    Ok(_) => {
                        reader_swallowed.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(_) => continue,
                }
            }
        });

        Self {
            port,
            swallowed,
            shutdown,
        }
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// Datagrams swallowed. Non-zero proves the client actually attempted QUIC.
    pub fn swallowed(&self) -> u64 {
        self.swallowed.load(Ordering::Relaxed)
    }
}

impl Drop for UdpBlackhole {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }
}
