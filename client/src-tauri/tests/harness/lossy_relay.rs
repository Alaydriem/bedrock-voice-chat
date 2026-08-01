use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use tokio::net::UdpSocket;
use tokio::sync::Mutex;

/// A UDP relay that discards a deterministic fraction of the server-to-client direction.
///
/// Deterministic rather than random: dropping every Nth datagram makes the induced rate exact, so an
/// assertion can be tight without being flaky. A pseudo-random drop would need a loose bound to
/// survive its own variance, which is the looseness that lets a broken derivation pass.
///
/// Only the downstream direction is touched. BVC carries voice in QUIC *datagrams*, which are
/// unreliable by design, so a discarded one is genuinely gone and the gap in the server's
/// per-connection sequence persists — which is the thing under test. Dropping upstream too would
/// perturb the client's own uplink loss detection and muddy which direction the assertion is about.
///
/// Multi-client, with one upstream socket per client address, the way a NAT behaves. A single shared
/// upstream would deliver every client's downstream traffic to whichever spoke last — and a test
/// needs two clients, because a lone client receives almost no stamped traffic: with nobody else
/// speaking the downstream is mostly acknowledgements, so a drop rarely costs a sequence number and
/// the induced loss never shows up.
///
/// Loss is gated behind `arm()` so handshakes and channel joins complete cleanly first.
pub struct LossyUdpRelay {
    port: u16,
    forwarded: Arc<AtomicU64>,
    dropped: Arc<AtomicU64>,
    armed: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
}

impl LossyUdpRelay {
    /// Binds `listen_port` facing clients, forwards to `server_quic_port`, and once armed discards one
    /// downstream datagram in every `drop_one_in` **per client**.
    pub async fn start(listen_port: u16, server_quic_port: u16, drop_one_in: u64) -> Self {
        assert!(drop_one_in >= 2, "drop_one_in must leave some traffic through");

        let client_sock = Arc::new(
            UdpSocket::bind((Ipv4Addr::LOCALHOST, listen_port))
                .await
                .expect("bind relay client-facing socket"),
        );
        let port = client_sock.local_addr().expect("relay local addr").port();

        let forwarded = Arc::new(AtomicU64::new(0));
        let dropped = Arc::new(AtomicU64::new(0));
        let armed = Arc::new(AtomicBool::new(false));
        let shutdown = Arc::new(AtomicBool::new(false));

        let relay = Self {
            port,
            forwarded: forwarded.clone(),
            dropped: dropped.clone(),
            armed: armed.clone(),
            shutdown: shutdown.clone(),
        };

        let server = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), server_quic_port);
        let upstreams: Arc<Mutex<HashMap<SocketAddr, Arc<UdpSocket>>>> =
            Arc::new(Mutex::new(HashMap::new()));

        let inbound_sock = client_sock.clone();
        let inbound_shutdown = shutdown.clone();
        tokio::spawn(async move {
            let mut buf = vec![0u8; 2048];

            while !inbound_shutdown.load(Ordering::Relaxed) {
                let Ok((len, from)) = inbound_sock.recv_from(&mut buf).await else {
                    continue;
                };

                let upstream = {
                    let mut map = upstreams.lock().await;
                    match map.get(&from) {
                        Some(existing) => existing.clone(),
                        None => {
                            let Ok(fresh) = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).await else {
                                continue;
                            };
                            let fresh = Arc::new(fresh);
                            map.insert(from, fresh.clone());

                            // One reader per client, so each client's downstream is counted and
                            // discarded independently.
                            Self::spawn_downstream(
                                fresh.clone(),
                                inbound_sock.clone(),
                                from,
                                drop_one_in,
                                armed.clone(),
                                forwarded.clone(),
                                dropped.clone(),
                                inbound_shutdown.clone(),
                            );

                            fresh
                        }
                    }
                };

                let _ = upstream.send_to(&buf[..len], server).await;
            }
        });

        relay
    }

    #[allow(clippy::too_many_arguments)]
    fn spawn_downstream(
        upstream: Arc<UdpSocket>,
        client_sock: Arc<UdpSocket>,
        client: SocketAddr,
        drop_one_in: u64,
        armed: Arc<AtomicBool>,
        forwarded: Arc<AtomicU64>,
        dropped: Arc<AtomicU64>,
        shutdown: Arc<AtomicBool>,
    ) {
        tokio::spawn(async move {
            let mut buf = vec![0u8; 2048];
            let mut seen: u64 = 0;

            while !shutdown.load(Ordering::Relaxed) {
                let Ok((len, _)) = upstream.recv_from(&mut buf).await else {
                    continue;
                };

                seen += 1;
                if armed.load(Ordering::Relaxed) && seen % drop_one_in == 0 {
                    dropped.fetch_add(1, Ordering::Relaxed);
                    continue;
                }

                forwarded.fetch_add(1, Ordering::Relaxed);
                let _ = client_sock.send_to(&buf[..len], client).await;
            }
        });
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// Begins discarding. Until called, every datagram is forwarded.
    pub fn arm(&self) {
        self.armed.store(true, Ordering::Relaxed);
    }

    pub fn forwarded(&self) -> u64 {
        self.forwarded.load(Ordering::Relaxed)
    }

    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    pub fn stop(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }
}

impl Drop for LossyUdpRelay {
    fn drop(&mut self) {
        self.stop();
    }
}
