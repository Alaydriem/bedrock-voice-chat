use std::net::ToSocketAddrs;
use std::sync::Arc;
use std::time::Instant;

use common::structs::relay::PeerCertResponse;

use super::dialer::PeerDialer;
use crate::relay::discovery::client::RelayClient;
use crate::relay::peer::link::ingest::PeerLinkIngest;
use crate::relay::peer::manager::PeerManager;
use crate::relay::peer_identity::ServerPeerStore;

// Seam the observe→redeem path drives once it holds a credential redeemed from
// the minter (the asker side of Flow 1). The credential is already in hand (from
// `/relay/peer-redeem`), so no peer-cert fetch is performed. Implementations must
// be non-blocking (spawn their own task); the QUIC input path never awaits I/O.
pub trait RedeemedDial: Send + Sync {
    fn dial_with_cert(&self, peer_ep: String, hashed_world: String, cred: PeerCertResponse);
}

// Production dialer for the asker side of Flow 1. Given a redeemed credential it
//   1. takes the link's outbound receiver from the `PeerManager` (so the queue
//      `forward_local` fills is drained);
//   2. resolves the acceptor's QUIC datagram port from its public `/api/config`;
//   3. constructs a `PeerDialer` with the redeemed credential and spawns its
//      bidirectional run loop (read pump -> GATED `PeerManager::ingest` via a
//      `PeerLinkIngest` bound to the peer endpoint; write pump -> connection).
//
// The peer endpoint advertised via discovery carries the acceptor's public HTTPS
// port. The `/api/config` lookup that resolves the QUIC datagram port targets that
// HTTPS port; only the final s2n-quic dial uses the resolved QUIC port. The peer
// identity / endpoint key / cert CN stay `host:{https_port}` so dial/accept dedup
// and tiebreak line up across the dialer, classifier, and acceptor.
pub struct ProductionPeerDialDriver {
    peer_manager: Arc<PeerManager>,
    server_peer_store: Arc<ServerPeerStore>,
}

impl ProductionPeerDialDriver {
    pub fn new(peer_manager: Arc<PeerManager>, server_peer_store: Arc<ServerPeerStore>) -> Self {
        Self {
            peer_manager,
            server_peer_store,
        }
    }

    pub fn new_shared(
        peer_manager: Arc<PeerManager>,
        server_peer_store: Arc<ServerPeerStore>,
    ) -> Arc<Self> {
        Arc::new(Self::new(peer_manager, server_peer_store))
    }

    // Splits a `host:{https_port}` endpoint key. Logs and returns `None` on a
    // malformed key.
    fn split_endpoint(peer_ep: &str) -> Option<(String, u16)> {
        match peer_ep.rsplit_once(':') {
            Some((h, p)) => match p.parse::<u16>() {
                Ok(port) => Some((h.to_string(), port)),
                Err(_) => {
                    tracing::warn!("relay dial: bad peer endpoint {}", peer_ep);
                    None
                }
            },
            None => {
                tracing::warn!("relay dial: peer endpoint missing port {}", peer_ep);
                None
            }
        }
    }

    // Takes the link's outbound receiver, resolves the acceptor's QUIC datagram
    // port from its public config, and pumps a bidirectional QUIC connection
    // through a gated `PeerLinkIngest` until close. The credential is the one
    // redeemed from the minter.
    async fn run_dial(
        peer_manager: Arc<PeerManager>,
        server_peer_store: Arc<ServerPeerStore>,
        peer_ep: String,
        host: String,
        http_port: u16,
        cred: PeerCertResponse,
    ) {
        // Take the outbound receiver BEFORE dialing so the write pump owns
        // the queue `forward_local` enqueues onto.
        let outbound_rx = match peer_manager.take_outbound_receiver(&peer_ep) {
            Some(rx) => rx,
            None => {
                tracing::debug!(
                    "relay dial: no outbound receiver for {} (already taken/closed)",
                    peer_ep
                );
                return;
            }
        };

        // Divine the QUIC datagram port from the acceptor's public config; the
        // dial below targets `host:{quic_port}` while identity stays HTTPS-keyed.
        let quic_port = match RelayClient::resolve_quic_port(&host, http_port).await {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!(
                    "relay dial: quic-port resolve failed for {}: {}",
                    peer_ep,
                    e
                );
                return;
            }
        };

        let socket = match (host.as_str(), quic_port).to_socket_addrs() {
            Ok(addrs) => {
                let resolved: Vec<_> = addrs.collect();
                // Prefer IPv4: a hostname like "localhost" resolves to `::1` first
                // on many systems, but BVC servers bind an IPv4 `listen` address,
                // so a v6 dial reaches no listener and the QUIC handshake times
                // out. Fall back to whatever resolved when no IPv4 exists.
                match resolved
                    .iter()
                    .find(|a| a.is_ipv4())
                    .or_else(|| resolved.first())
                {
                    Some(addr) => *addr,
                    None => {
                        tracing::warn!("relay dial: no socket addr for {}", peer_ep);
                        return;
                    }
                }
            }
            Err(e) => {
                tracing::warn!("relay dial: resolve {} failed: {}", peer_ep, e);
                return;
            }
        };

        let dialer = PeerDialer::new(
            cred.ca_pem.into_bytes(),
            cred.cert_pem.into_bytes(),
            cred.key_pem.into_bytes(),
        );

        // GATED ingest bound to this peer endpoint: the dialer's read pump
        // routes inbound datagrams through `PeerManager::ingest`, so the
        // presence gate applies to the initiator path exactly as it does to the
        // acceptor path.
        let gated_ingest = PeerLinkIngest::new_shared(peer_manager.clone(), peer_ep.clone());

        // Opens the QUIC connection and pumps datagrams until close.
        if let Err(e) = dialer.run(socket, host, gated_ingest, outbound_rx).await {
            tracing::warn!("relay peer dial to {} ended: {:#}", peer_ep, e);
        }

        // Connection closed (abrupt drop or graceful): drop the link now and start
        // the identity's reconnect grace, so the asker re-offers once grace lapses
        // instead of waiting out the multi-minute idle sweep.
        peer_manager.drop_link(&peer_ep);
        server_peer_store.mark_disconnected(
            &peer_ep,
            Instant::now(),
            ServerPeerStore::RECONNECT_GRACE,
        );
    }
}

impl RedeemedDial for ProductionPeerDialDriver {
    fn dial_with_cert(&self, peer_ep: String, _hashed_world: String, cred: PeerCertResponse) {
        let peer_manager = self.peer_manager.clone();
        let server_peer_store = self.server_peer_store.clone();
        tokio::spawn(async move {
            let (host, http_port) = match Self::split_endpoint(&peer_ep) {
                Some(parts) => parts,
                None => return,
            };
            Self::run_dial(
                peer_manager,
                server_peer_store,
                peer_ep,
                host,
                http_port,
                cred,
            )
            .await;
        });
    }
}
