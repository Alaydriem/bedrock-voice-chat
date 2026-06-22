use std::sync::Arc;

use common::structs::packet::QuicNetworkPacket;

use super::ingest_sink::GatedPeerIngest;
use crate::relay::peer::manager::PeerManager;

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
    use crate::relay::peer::link::ingest_sink::RelayIngestSink;
    use crate::relay::peer::table::PeerTable;
    use crate::relay::presence::gate::{AlwaysProven, NeverProven};
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
        use common::structs::packet::{AudioFramePacket, PacketType, QuicNetworkPacketData};
        use common::{Coordinate, Orientation, PlayerEnum};
        let sender = PlayerEnum::Minecraft(MinecraftPlayer {
            name: "alice".into(),
            coordinates: Coordinate {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
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

    // The dialer-side gated ingest mirrors the acceptor gate: an unauthorized
    // peer's relayed AUDIO is dropped fail-closed, while an authorized peer's is
    // published. (Per-world authorization transitions are covered by the
    // link→world gate tests once that gate is in place.)
    #[tokio::test]
    async fn dialer_ingest_drops_unauthorized_publishes_authorized() {
        let key = PeerManager::endpoint_key(&ep("peerX", 7000));

        // Unauthorized gate: dialer-received audio is dropped.
        let dropped = Arc::new(SpySink {
            published: AtomicUsize::new(0),
        });
        let mgr = Arc::new(PeerManager::new(
            ep("self", 1),
            PeerTable::new_shared(),
            dropped.clone(),
            Arc::new(NeverProven),
        ));
        mgr.register_inbound(&key, std::time::Instant::now());
        let gated = PeerLinkIngest::new(mgr.clone(), key.clone());
        gated.ingest_from_peer(audio_packet_in_world("W")).await;
        assert_eq!(
            dropped.published.load(Ordering::SeqCst),
            0,
            "dialer-received audio from an unauthorized peer must be dropped (gated)"
        );

        // Authorized gate: dialer-received audio is published via the same path.
        let published = Arc::new(SpySink {
            published: AtomicUsize::new(0),
        });
        let mgr2 = Arc::new(PeerManager::new(
            ep("self", 1),
            PeerTable::new_shared(),
            published.clone(),
            Arc::new(AlwaysProven),
        ));
        mgr2.register_inbound(&key, std::time::Instant::now());
        let gated2 = PeerLinkIngest::new(mgr2.clone(), key.clone());
        gated2.ingest_from_peer(audio_packet_in_world("W")).await;
        assert_eq!(
            published.published.load(Ordering::SeqCst),
            1,
            "an authorized peer's dialer-received audio is published"
        );
    }
}
