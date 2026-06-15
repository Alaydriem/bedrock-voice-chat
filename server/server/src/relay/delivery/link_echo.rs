use std::sync::Arc;

use common::structs::packet::{
    PacketType, PeerPresenceObservedPacket, QuicNetworkPacket, QuicNetworkPacketData,
};

use super::super::orchestrator::PeerEchoDelivery;
use super::super::relayed_packet::RelayedPacket;
use crate::relay::peer::manager::PeerManager;
use crate::relay::presence::PresenceProver;

// Production `PeerEchoDelivery`: enqueues a `PeerPresenceObserved` onto every
// live peer link's outbound queue (the same datagram channel the peer-link
// writer drains), and records — per peer it reached — our half of the mutual
// proof. Bounded `try_send` with drop-on-full inside `enqueue_to_all_links`, so
// the echo never blocks.
pub struct LinkEchoDelivery {
    peer_manager: Arc<PeerManager>,
    prover: Arc<PresenceProver>,
}

impl LinkEchoDelivery {
    pub fn new(peer_manager: Arc<PeerManager>, prover: Arc<PresenceProver>) -> Self {
        Self {
            peer_manager,
            prover,
        }
    }

    pub fn new_shared(peer_manager: Arc<PeerManager>, prover: Arc<PresenceProver>) -> Arc<Self> {
        Arc::new(Self::new(peer_manager, prover))
    }
}

impl PeerEchoDelivery for LinkEchoDelivery {
    fn echo_observed(&self, token: &str, hashed_world: &str) {
        let observed = QuicNetworkPacket {
            packet_type: PacketType::PeerPresenceObserved,
            owner: None,
            data: QuicNetworkPacketData::PeerPresenceObserved(PeerPresenceObservedPacket {
                token: token.to_string(),
            }),
        };
        let reached = self
            .peer_manager
            .enqueue_to_all_links(&RelayedPacket::local(observed));
        for peer in reached {
            // Record our half of the mutual proof for the world this observed
            // token was attributed to (world-scoped).
            self.prover.record_echoed_to_peer(&peer, hashed_world);
        }
    }
}
