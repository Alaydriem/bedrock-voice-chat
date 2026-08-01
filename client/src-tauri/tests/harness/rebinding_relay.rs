use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use tokio::net::UdpSocket;

/// A UDP relay that periodically rebinds its upstream socket, so the server observes
/// the same QUIC connection arriving from a fresh source port each time.
///
/// This is what a carrier NAT64/CLAT translator pool does to a client, and it is the
/// mechanism behind the reported failure: s2n-quic identifies a path by remote
/// address *including port*, allows five paths per connection
/// (`s2n-quic-transport::path::manager::MAX_ALLOWED_PATHS`), and never reclaims one.
/// Once the budget is spent every further datagram is dropped with
/// `PathLimitExceeded` until idle timeout.
///
/// Rotation is gated behind `arm()` so the handshake completes on a stable path
/// first; a translator that rotated mid-handshake would produce a different failure.
pub struct RebindingUdpRelay {
    port: u16,
    downstream_datagrams: Arc<AtomicU64>,
    upstream_datagrams: Arc<AtomicU64>,
    rebinds: Arc<AtomicU64>,
    armed: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
}

impl RebindingUdpRelay {
    /// Binds `listen_port` facing the client and forwards to `server_quic_port`,
    /// rebinding the upstream socket every `rotate_every` once armed.
    pub async fn start(listen_port: u16, server_quic_port: u16, rotate_every: Duration) -> Self {
        let client_sock = UdpSocket::bind((Ipv4Addr::LOCALHOST, listen_port))
            .await
            .expect("bind relay client-facing socket");
        let port = client_sock.local_addr().expect("relay local addr").port();

        let downstream_datagrams = Arc::new(AtomicU64::new(0));
        let upstream_datagrams = Arc::new(AtomicU64::new(0));
        let rebinds = Arc::new(AtomicU64::new(0));
        let armed = Arc::new(AtomicBool::new(false));
        let shutdown = Arc::new(AtomicBool::new(false));

        let relay = Self {
            port,
            downstream_datagrams: downstream_datagrams.clone(),
            upstream_datagrams: upstream_datagrams.clone(),
            rebinds: rebinds.clone(),
            armed: armed.clone(),
            shutdown: shutdown.clone(),
        };

        tokio::spawn(async move {
            let server = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), server_quic_port);
            let mut upstream = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0))
                .await
                .expect("bind relay upstream socket");

            let mut client_addr: Option<SocketAddr> = None;
            let mut from_client = vec![0u8; 2048];
            let mut from_server = vec![0u8; 2048];
            let mut ticker = tokio::time::interval(rotate_every);
            ticker.tick().await;

            while !shutdown.load(Ordering::Relaxed) {
                tokio::select! {
                    received = client_sock.recv_from(&mut from_client) => {
                        if let Ok((len, from)) = received {
                            client_addr = Some(from);
                            upstream_datagrams.fetch_add(1, Ordering::Relaxed);
                            let _ = upstream.send_to(&from_client[..len], server).await;
                        }
                    }
                    received = upstream.recv_from(&mut from_server) => {
                        if let Ok((len, _)) = received {
                            downstream_datagrams.fetch_add(1, Ordering::Relaxed);
                            if let Some(to) = client_addr {
                                let _ = client_sock.send_to(&from_server[..len], to).await;
                            }
                        }
                    }
                    _ = ticker.tick() => {
                        if armed.load(Ordering::Relaxed) {
                            // A fresh ephemeral port is what the server reads as a
                            // new path. Binding a brand new socket each time also
                            // guarantees no rotation reuses an address the server
                            // has already admitted.
                            if let Ok(fresh) = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).await {
                                upstream = fresh;
                                rebinds.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    }
                }
            }
        });

        relay
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// Begins source-address rotation. Called once the connection is established so
    /// the handshake itself is not what breaks.
    pub fn arm(&self) {
        self.armed.store(true, Ordering::Relaxed);
    }

    /// Datagrams the server has sent back. The signal this test reads: a live QUIC
    /// connection produces a steady return flow (ACKs, keep-alive responses), and a
    /// connection whose path budget is exhausted produces none, because the server
    /// discards the client's datagrams before they reach it.
    pub fn downstream_datagrams(&self) -> u64 {
        self.downstream_datagrams.load(Ordering::Relaxed)
    }

    pub fn upstream_datagrams(&self) -> u64 {
        self.upstream_datagrams.load(Ordering::Relaxed)
    }

    pub fn rebinds(&self) -> u64 {
        self.rebinds.load(Ordering::Relaxed)
    }

    /// Blocks until at least `want` rotations have happened or the timeout elapses.
    pub async fn await_rebinds(&self, want: u64, timeout: Duration) -> Result<(), String> {
        let deadline = tokio::time::Instant::now() + timeout;
        while tokio::time::Instant::now() < deadline {
            if self.rebinds() >= want {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        Err(format!(
            "only {} of {} rebinds happened within {:?}",
            self.rebinds(),
            want,
            timeout
        ))
    }

    pub fn stop(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }
}

impl Drop for RebindingUdpRelay {
    fn drop(&mut self) {
        self.stop();
    }
}
