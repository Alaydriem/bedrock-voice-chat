use common::curia;
use std::sync::Arc;

use common::structs::packet::QuicNetworkPacket;
use iroh::{EndpointAddr, PublicKey};

use bvc_relay::node::NodeIdentity;
use bvc_relay::peer::{AdmissionControl, Handshake, PeerEndpoint, PeerError, PeerLink};

use crate::relay::grant::GrantTable;
use crate::relay::peer::AdvertisedAddress;

use super::egress::PeerEgress;
use super::ingest::PeerIngest;
use super::links::PeerLinks;
use super::local_clients::LocalClients;
use super::sink::PeerSink;

// The peer plane: one endpoint, every live link, and both directions of traffic.
//
// Everything inbound passes the ingest boundary before it reaches the sink, and
// everything outbound is scoped to the sender's relay world. A peer's frames are
// published straight to the sink and never re-enter the outbound path, which is
// what keeps relay single-hop.
pub struct PeerPlane {
    endpoint: PeerEndpoint,
    links: PeerLinks,
    grants: Arc<GrantTable>,
    ingest: PeerIngest,
    admission: AdmissionControl,
    sink: Arc<dyn PeerSink>,
    // Where a relayed speaker's position is published, so audio routing resolves it the same
    // way it resolves a local player's. Written per frame because a relayed peer moves, and
    // the cache's own presence TTL is what ages a silent one out.
    speakers: Arc<moka::future::Cache<String, common::PlayerEnum>>,
    // The minted ticket, so `/api/config` can serve it on every request without paying a
    // registry round trip each time. One entry because there is one ticket; a TTL rather
    // than a permanent cell so a server whose observed address changes picks the new one up
    // without a restart.
    ticket: moka::future::Cache<(), String>,
}

impl PeerPlane {
    // Far above any real topology and far below anything worth exhausting file
    // descriptors over.
    const MAX_UNAUTHORIZED: usize = 64;

    // Long enough that a polled endpoint costs one observation rather than thousands,
    // short enough that a server whose public address moved is not advertising a dead one
    // for the rest of its uptime.
    const TICKET_TTL: std::time::Duration = std::time::Duration::from_secs(300);

    pub async fn bind(
        identity: &NodeIdentity,
        grants: Arc<GrantTable>,
        locals: Arc<dyn LocalClients>,
        sink: Arc<dyn PeerSink>,
        speakers: Arc<moka::future::Cache<String, common::PlayerEnum>>,
        // `server.peer_port`. Absent leaves the port to the operating system, which
        // is a different one on every start — and this endpoint's port is part of
        // the ticket an operator pastes into the far side's config.
        port: Option<u16>,
    ) -> Result<Arc<Self>, PeerError> {
        let endpoint = PeerEndpoint::bind_on(identity, port).await?;

        Ok(Arc::new(Self {
            endpoint,
            links: PeerLinks::new(),
            ingest: PeerIngest::new(Arc::clone(&grants), locals),
            grants,
            admission: AdmissionControl::new(Self::MAX_UNAUTHORIZED),
            sink,
            speakers,
            ticket: moka::future::Cache::builder()
                .time_to_live(Self::TICKET_TTL)
                .max_capacity(1)
                .build(),
        }))
    }

    /// The peer link, minted once and reused until its TTL lapses.
    ///
    /// `/api/config` is polled, and `ticket_observed` spends a registry round trip bounded
    /// by `AddressObserver::TIMEOUT`. Paying that per request would make an unreachable
    /// registry into a ten-second stall on an endpoint every client calls.
    ///
    /// A failure is not cached: a registry that was briefly down would otherwise leave this
    /// server advertising nothing for the whole TTL.
    pub async fn cached_ticket(
        &self,
        registry: Option<String>,
        peer_port: Option<u16>,
    ) -> Result<String, PeerError> {
        if let Some(cached) = self.ticket.get(&()).await {
            return Ok(cached);
        }

        let ticket = self.ticket_observed(registry, peer_port).await?;
        self.ticket.insert((), ticket.clone()).await;

        Ok(ticket)
    }

    /// A ticket carrying the address the registry saw this server at.
    ///
    /// Observed lazily, when an operator asks for a peer link rather than at startup:
    /// a registry that is down then costs one command instead of a boot. An
    /// observation that fails is not fatal — the ticket still carries the locally
    /// observed addresses, which is everything a same-host or LAN peer needs.
    pub async fn ticket_observed(
        &self,
        registry: Option<String>,
        peer_port: Option<u16>,
    ) -> Result<String, PeerError> {
        let advertised =
            AdvertisedAddress::from_observation(self.observe(registry).await, peer_port);

        self.endpoint
            .ticket_advertising(advertised)
            .await
            .map_err(|e| PeerError::Transport(e.to_string()))
    }

    // A failed observation is reported and dropped rather than raised. An operator
    // asking for a peer link on an isolated host has no registry to reach and still
    // needs the ticket.
    async fn observe(&self, registry: Option<String>) -> Option<std::net::SocketAddr> {
        let peerlink = registry?;
        let addr = match bvc_relay::node::PeerTicket::parse(&peerlink) {
            Ok(addr) => addr,
            Err(e) => {
                curia::warn!(format!("the configured registry is not a peer link: {e}"));
                return None;
            }
        };

        match bvc_relay::peer::AddressObserver::observe(self.endpoint.endpoint(), addr).await {
            Ok(observed) => observed,
            Err(e) => {
                curia::warn!(format!(
                    "could not ask the registry for this server's address: {e}"
                ));
                None
            }
        }
    }

    /// The table this plane authorizes against.
    ///
    /// Exposed so a revocation reaching the HTTP surface can drop the grant from the
    /// running table as well as the row. Without it a revoked bridge keeps its
    /// authorization until the process restarts.
    pub fn grants(&self) -> &Arc<GrantTable> {
        &self.grants
    }

    pub fn node_id(&self) -> PublicKey {
        self.endpoint.node_id()
    }

    pub fn endpoint(&self) -> &PeerEndpoint {
        &self.endpoint
    }

    pub fn links(&self) -> &PeerLinks {
        &self.links
    }

    // Sends a local-origin packet to every peer granted the sender's relay world.
    // Returns how many took it; zero is the ordinary answer for traffic that is
    // not peer traffic.
    pub fn forward_local(
        &self,
        packet: &QuicNetworkPacket,
        speaker: &common::PlayerEnum,
    ) -> usize {
        match PeerEgress::frame_from(packet, speaker) {
            Some((world, frame)) => self.links.broadcast_world(&world, &frame),
            None => 0,
        }
    }

    pub async fn dial(
        self: &Arc<Self>,
        addr: EndpointAddr,
        worlds: Vec<String>,
    ) -> Result<(), PeerError> {
        let conn = self
            .endpoint
            .endpoint()
            .connect(addr, PeerEndpoint::ALPN)
            .await
            .map_err(|e| PeerError::Transport(e.to_string()))?;

        let accepted = Handshake::dial(&conn, worlds).await?;
        let link = PeerLink::establish(conn, accepted.worlds)?;

        self.spawn_pump(link);
        Ok(())
    }

    pub fn spawn_accept_loop(self: &Arc<Self>) {
        let plane = Arc::clone(self);
        tokio::spawn(async move {
            while let Some(incoming) = plane.endpoint.endpoint().accept().await {
                let plane = Arc::clone(&plane);
                tokio::spawn(async move {
                    plane.admit(incoming).await;
                });
            }
        });
    }

    // The admission slot is held only until the peer is authorized: it bounds the
    // unauthorized window, not the lifetime of a legitimate link.
    async fn admit(self: &Arc<Self>, incoming: iroh::endpoint::Incoming) {
        let Some(slot) = self.admission.try_admit() else {
            curia::warn!("refusing a peer connection: too many unauthorized in flight");
            return;
        };

        let conn = match incoming.await {
            Ok(conn) => conn,
            Err(e) => {
                curia::debug!(format!("a peer connection failed before the handshake: {e}"));
                return;
            }
        };

        let handshake = tokio::time::timeout(
            AdmissionControl::PRE_AUTH_TIMEOUT,
            Handshake::accept(&conn, self.grants.as_ref()),
        )
        .await;

        let accepted = match handshake {
            Ok(Ok(accepted)) => accepted,
            Ok(Err(e)) => {
                curia::warn!(format!("refusing a peer: {e}"));
                return;
            }
            Err(_) => {
                curia::warn!(
                    "refusing a peer that did not finish the handshake within the pre-auth timeout"
                );
                return;
            }
        };

        match PeerLink::establish(conn, accepted.worlds) {
            Ok(link) => {
                let label = self
                    .grants
                    .grant_for(&link.node())
                    .map(|grant| grant.label().to_string())
                    .unwrap_or_default();
                curia::info!("peer link established", { "peer": label.to_string(), "node": link.node().to_string(), "worlds": format!("{:?}", link.worlds()) });
                drop(slot);
                self.spawn_pump(link);
            }
            Err(e) => curia::warn!(format!("refusing a peer link: {e}")),
        }
    }

    // Registers the link, then reads its datagrams until it closes, admitting
    // each through the ingest boundary.
    //
    // The insert happens here rather than inside the task: a caller that dials
    // and then immediately forwards would otherwise race the spawn and find an
    // empty table, dropping audio for as long as the scheduler took.
    fn spawn_pump(self: &Arc<Self>, link: PeerLink) {
        // Two bridges fronting one logical world is legitimate, so this is not a
        // refusal. It is also what a second, misconfigured bridge looks like, and
        // in that case every frame in the world arrives twice.
        let overlapping = self.links.worlds_also_carried(&link);
        if !overlapping.is_empty() {
            curia::error!("a second peer declares worlds already carried; audio in them will reach both", { "node": link.node().to_string(), "worlds": format!("{overlapping:?}") });
        }

        self.links.insert(link.clone());

        let plane = Arc::clone(self);
        tokio::spawn(async move {
            let node = link.node();

            loop {
                let frame = match link.recv().await {
                    Ok(frame) => frame,
                    Err(e) => {
                        curia::info!(format!("peer link ended: {e}"), { "node": node.to_string() });
                        break;
                    }
                };

                match plane.ingest.admit(&node, frame) {
                    Ok((packet, speaker)) => {
                        // Published before the packet, so routing never sees a relayed frame
                        // whose speaker it cannot resolve.
                        if let Some(key) = packet.sender_key() {
                            plane.speakers.insert(key, speaker).await;
                        }
                        plane.sink.publish(packet);
                    }
                    Err(rejection) => {
                        curia::warn!(format!("dropping a peer frame: {rejection}"), { "node": node.to_string() })
                    }
                }
            }

            plane.links.remove(&node);
        });
    }
}
