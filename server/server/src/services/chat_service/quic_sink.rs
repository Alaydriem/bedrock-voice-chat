use std::sync::Arc;

use common::PlayerEnum;
use common::structs::packet::{
    ChatMessagePacket, PacketType, QuicNetworkPacket, QuicNetworkPacketData,
};

use super::sink::ChatSink;
use crate::stream::quic::connection_registry::ConnectionRegistry;

/// Delivers a chat line to every voice connection standing in that world, and to nobody else.
///
/// `ConnectionRegistry::broadcast_to_all` is deliberately not used: it is not world-scoped and
/// would cross-post between worlds on a multi-world server. `forward_local_to_peers` is not
/// used either — peers are separate instances, and a chat room is one world on one server.
pub struct QuicChatSink {
    registry: Arc<ConnectionRegistry>,
    players: Arc<moka::future::Cache<String, PlayerEnum>>,
}

impl QuicChatSink {
    pub fn new(
        registry: Arc<ConnectionRegistry>,
        players: Arc<moka::future::Cache<String, PlayerEnum>>,
    ) -> Self {
        Self { registry, players }
    }

    pub fn new_shared(
        registry: Arc<ConnectionRegistry>,
        players: Arc<moka::future::Cache<String, PlayerEnum>>,
    ) -> Arc<Self> {
        Arc::new(Self::new(registry, players))
    }
}

impl ChatSink for QuicChatSink {
    fn deliver(&self, world_uuid: &str, packet: &ChatMessagePacket) {
        let outbound = QuicNetworkPacket {
            packet_type: PacketType::ChatMessage,
            sender: None,
            data: QuicNetworkPacketData::ChatMessage(packet.clone()),
            ..Default::default()
        };

        let mut delivered = 0usize;
        for (identity, player) in self.players.iter() {
            let PlayerEnum::Minecraft(mc) = &player else {
                continue;
            };
            if mc.world_uuid.as_deref() != Some(world_uuid) {
                continue;
            }
            if self.registry.send_to_player(identity.as_str(), &outbound) {
                delivered += 1;
            }
        }

        tracing::debug!(world = %world_uuid, delivered, "chat fanned out");
    }
}
