use std::sync::Arc;

use common::structs::packet::QuicNetworkPacket;
use iroh::{EndpointAddr, PublicKey, RelayUrl};

use bvc_relay::node::NodeIdentity;
use bvc_relay::peer::{AdmissionControl, Handshake, PeerEndpoint, PeerError, PeerLink};

use crate::relay::grant::GrantTable;

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
}

impl PeerPlane {
    // Far above any real topology and far below anything worth exhausting file
    // descriptors over.
    const MAX_UNAUTHORIZED: usize = 64;

    pub async fn bind(
        identity: &NodeIdentity,
        grants: Arc<GrantTable>,
        locals: Arc<dyn LocalClients>,
        sink: Arc<dyn PeerSink>,
        relay_url: Option<RelayUrl>,
        // `server.peer_port`. Absent leaves the port to the operating system, which
        // is a different one on every start — and this endpoint's port is part of
        // the ticket an operator pastes into the far side's config.
        port: Option<u16>,
    ) -> Result<Arc<Self>, PeerError> {
        let endpoint = PeerEndpoint::bind_on(identity, relay_url, port).await?;

        Ok(Arc::new(Self {
            endpoint,
            links: PeerLinks::new(),
            ingest: PeerIngest::new(Arc::clone(&grants), locals),
            grants,
            admission: AdmissionControl::new(Self::MAX_UNAUTHORIZED),
            sink,
        }))
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
    pub fn forward_local(&self, packet: &QuicNetworkPacket) -> usize {
        match PeerEgress::frame_from(packet) {
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
            tracing::warn!("refusing a peer connection: too many unauthorized in flight");
            return;
        };

        let conn = match incoming.await {
            Ok(conn) => conn,
            Err(e) => {
                tracing::debug!("a peer connection failed before the handshake: {e}");
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
                tracing::warn!("refusing a peer: {e}");
                return;
            }
            Err(_) => {
                tracing::warn!(
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
                tracing::info!(
                    peer = %label,
                    node = %link.node(),
                    worlds = ?link.worlds(),
                    "peer link established"
                );
                drop(slot);
                self.spawn_pump(link);
            }
            Err(e) => tracing::warn!("refusing a peer link: {e}"),
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
            tracing::error!(
                node = %link.node(),
                worlds = ?overlapping,
                "a second peer declares worlds already carried; audio in them will reach both"
            );
        }

        self.links.insert(link.clone());

        let plane = Arc::clone(self);
        tokio::spawn(async move {
            let node = link.node();

            loop {
                let frame = match link.recv().await {
                    Ok(frame) => frame,
                    Err(e) => {
                        tracing::info!(node = %node, "peer link ended: {e}");
                        break;
                    }
                };

                match plane.ingest.admit(&node, frame) {
                    Ok(packet) => plane.sink.publish(packet),
                    Err(rejection) => {
                        tracing::warn!(node = %node, "dropping a peer frame: {rejection}")
                    }
                }
            }

            plane.links.remove(&node);
        });
    }
}
