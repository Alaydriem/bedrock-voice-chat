use std::sync::Arc;

use common::structs::packet::QuicNetworkPacket;

use super::ingest_sink::GatedPeerIngest;
use super::peer_manager::PeerManager;

// Binds a `PeerManager` together with a single peer endpoint so a peer-link read
// pump can route every inbound datagram through the GATED `PeerManager::ingest`
// without knowing the endpoint at each call. Used by the dialer
// (initiator) read pump so its inbound packets pass the SAME presence-proof gate
// the acceptor path already applies — closing the half-gated bypass.
pub struct PeerLinkIngest {
    manager: Arc<PeerManager>,
    endpoint: String,
}

impl PeerLinkIngest {
    pub fn new(manager: Arc<PeerManager>, endpoint: String) -> Self {
        Self { manager, endpoint }
    }

    pub fn new_shared(manager: Arc<PeerManager>, endpoint: String) -> Arc<Self> {
        Arc::new(Self::new(manager, endpoint))
    }
}

#[async_trait::async_trait]
impl GatedPeerIngest for PeerLinkIngest {
    async fn ingest_from_peer(&self, packet: QuicNetworkPacket) {
        self.manager.ingest(&self.endpoint, packet).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::relay::ingest_sink::RelayIngestSink;
    use crate::services::relay::peer_table::PeerTable;
    use crate::services::relay::presence::PresenceProver;
    use common::structs::relay::RelayEndpoint;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn ep(host: &str, port: u16) -> RelayEndpoint {
        RelayEndpoint {
            host: host.into(),
            port,
            primary: false,
        }
    }

    struct SpySink {
        published: AtomicUsize,
    }

    #[async_trait::async_trait]
    impl RelayIngestSink for SpySink {
        async fn publish(&self, _packet: QuicNetworkPacket) {
            self.published.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn audio_packet_in_world(relay_world: &str) -> QuicNetworkPacket {
        use common::game_data::Dimension;
        use common::players::MinecraftPlayer;
        use common::structs::packet::{
            AudioFramePacket, PacketType, QuicNetworkPacketData,
        };
        use common::{Coordinate, Orientation, PlayerEnum};
        let sender = PlayerEnum::Minecraft(MinecraftPlayer {
            name: "alice".into(),
            coordinates: Coordinate { x: 0.0, y: 0.0, z: 0.0 },
            orientation: Orientation { x: 0.0, y: 0.0 },
            dimension: Dimension::Overworld,
            deafen: false,
            spectator: false,
            world_uuid: None,
            alternative_identity: None,
            player_uuid: None,
            relay_world_uuid: Some(relay_world.into()),
        });
        QuicNetworkPacket {
            packet_type: PacketType::AudioFrame,
            owner: None,
            data: QuicNetworkPacketData::AudioFrame(AudioFramePacket::new(
                vec![9, 9, 9],
                48000,
                Some(sender),
                Some(true),
            )),
        }
    }

    // The dialer-side gated ingest mirrors the acceptor gate. An un-proven peer's
    // relayed AUDIO is dropped fail-closed; once the peer is
    // mutually proven for the packet's world the SAME packet is published.
    #[tokio::test]
    async fn dialer_ingest_drops_audio_from_unproven_peer_then_publishes_once_proven() {
        let sink = Arc::new(SpySink {
            published: AtomicUsize::new(0),
        });
        let prover = PresenceProver::new_shared();
        let peer = ep("peerX", 7000);
        let key = PeerManager::endpoint_key(&peer);
        let mgr = Arc::new(PeerManager::new(
            ep("self", 1),
            PeerTable::new_shared(),
            sink.clone(),
            prover.clone(),
        ));
        mgr.set_prover(prover.clone());
        mgr.register_inbound(&key, std::time::Instant::now());

        let gated = PeerLinkIngest::new(mgr.clone(), key.clone());

        // Un-proven: dropped.
        gated.ingest_from_peer(audio_packet_in_world("W")).await;
        assert_eq!(
            sink.published.load(Ordering::SeqCst),
            0,
            "dialer-received audio from an un-proven peer must be dropped (gated)"
        );

        // Complete the mutual proof for world W against this peer.
        let now = std::time::Instant::now();
        let token = prover.new_challenge("W", now);
        prover.record_observed_from_peer(&key, &token, now);
        prover.record_echoed_to_peer(&key, "W");

        gated.ingest_from_peer(audio_packet_in_world("W")).await;
        assert_eq!(
            sink.published.load(Ordering::SeqCst),
            1,
            "once mutually proven, dialer-received audio is published"
        );
    }
}
