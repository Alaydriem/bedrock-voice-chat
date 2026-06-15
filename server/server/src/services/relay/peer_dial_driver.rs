use std::net::ToSocketAddrs;
use std::sync::Arc;

use common::structs::relay::RelayEndpoint;

use super::client::RelayClient;
use super::peer_dialer::PeerDialer;
use super::peer_link_ingest::PeerLinkIngest;
use super::peer_manager::PeerManager;

// Seam the orchestrator drives for each dial intent `reconcile` produces.
pub trait PeerDialDriver: Send + Sync {
    // Begin dialing `peer_ep` for `hashed_world`. Implementations must be
    // non-blocking (spawn their own task); the orchestrator never awaits I/O.
    fn begin_dial(&self, peer_ep: String, hashed_world: String);
}

// Production driver: for each dial intent it
//   1. fetches an in-memory peer cert from the acceptor via the SPKI-pinned
//      `RelayClient` (`fetch_peer_cert`), which the acceptor issues only on a
//      completed mutual presence proof;
//   2. takes the link's outbound receiver from the `PeerManager` (so the queue
//      `forward_local` fills is drained);
//   3. constructs a `PeerDialer` with the issued credential and spawns its
//      bidirectional run loop (read pump -> GATED `PeerManager::ingest` via a
//      `PeerLinkIngest` bound to the peer endpoint; write pump -> connection).
//
// The peer endpoint advertised via discovery carries the acceptor's public HTTPS
// port. Both the peer-cert fetch and the `/api/config` lookup that resolves the
// QUIC datagram port target that HTTPS port; only the final s2n-quic dial uses
// the resolved QUIC port. The peer identity / endpoint key / cert CN stay
// `host:{https_port}` so dial/accept dedup and tiebreak line up across the
// dialer, classifier, and acceptor.
pub struct ProductionPeerDialDriver {
    relay_client: Arc<RelayClient>,
    peer_manager: Arc<PeerManager>,
    self_endpoint: RelayEndpoint,
}

impl ProductionPeerDialDriver {
    pub fn new(
        relay_client: Arc<RelayClient>,
        peer_manager: Arc<PeerManager>,
        self_endpoint: RelayEndpoint,
    ) -> Self {
        Self {
            relay_client,
            peer_manager,
            self_endpoint,
        }
    }

    pub fn new_shared(
        relay_client: Arc<RelayClient>,
        peer_manager: Arc<PeerManager>,
        self_endpoint: RelayEndpoint,
    ) -> Arc<Self> {
        Arc::new(Self::new(relay_client, peer_manager, self_endpoint))
    }
}

impl PeerDialDriver for ProductionPeerDialDriver {
    fn begin_dial(&self, peer_ep: String, hashed_world: String) {
        let relay_client = self.relay_client.clone();
        let peer_manager = self.peer_manager.clone();
        let self_endpoint = self.self_endpoint.clone();

        tokio::spawn(async move {
            // The peer endpoint key is `host:{https_port}`.
            let (host, http_port) = match peer_ep.rsplit_once(':') {
                Some((h, p)) => match p.parse::<u16>() {
                    Ok(port) => (h.to_string(), port),
                    Err(_) => {
                        tracing::warn!("relay dial: bad peer endpoint {}", peer_ep);
                        return;
                    }
                },
                None => {
                    tracing::warn!("relay dial: peer endpoint missing port {}", peer_ep);
                    return;
                }
            };

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

            // Fetch the in-memory peer cert from the acceptor's HTTPS port (gated
            // on mutual presence proof on the acceptor side).
            let cred = match relay_client
                .fetch_peer_cert(&self_endpoint, &host, http_port, &hashed_world)
                .await
            {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!("relay dial: peer-cert fetch failed for {}: {}", peer_ep, e);
                    return;
                }
            };

            // Divine the QUIC datagram port from the acceptor's public config; the
            // dial below targets `host:{quic_port}` while identity stays HTTPS-keyed.
            let quic_port = match RelayClient::resolve_quic_port(&host, http_port).await {
                Ok(p) => p,
                Err(e) => {
                    tracing::warn!("relay dial: quic-port resolve failed for {}: {}", peer_ep, e);
                    return;
                }
            };

            let socket = match (host.as_str(), quic_port).to_socket_addrs() {
                Ok(mut addrs) => match addrs.next() {
                    Some(addr) => addr,
                    None => {
                        tracing::warn!("relay dial: no socket addr for {}", peer_ep);
                        return;
                    }
                },
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
            // presence-proof gate applies to the initiator path exactly as it
            // does to the acceptor path.
            let gated_ingest = PeerLinkIngest::new_shared(peer_manager.clone(), peer_ep.clone());

            // Opens the QUIC connection and pumps datagrams until close.
            if let Err(e) = dialer.run(socket, host, gated_ingest, outbound_rx).await {
                tracing::warn!("relay peer dial to {} ended: {}", peer_ep, e);
            }
        });
    }
}
