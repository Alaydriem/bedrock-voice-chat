use std::sync::Arc;

use common::structs::packet::{
    PacketType, PeerAnnounceInjectPacket, PeerPresenceInjectPacket, QuicNetworkPacket,
    QuicNetworkPacketData,
};

use crate::stream::quic::WebhookReceiver;

use super::super::orchestrator::LocalInjectDelivery;

// Production `LocalInjectDelivery`: broadcasts a `PeerPresenceInject` to local
// clients via the same webhook/broadcast path everything else uses. A client
// only injects the token into the realm it is actually proxying, so broadcasting
// is safe — non-participating clients have no matching realm to inject into. The
// challenge token therefore reaches the realm ONLY through a local client; it is
// never placed on a peer link here.
pub struct BroadcastInjectDelivery {
    webhook: WebhookReceiver,
}

impl BroadcastInjectDelivery {
    pub fn new(webhook: WebhookReceiver) -> Self {
        Self { webhook }
    }

    pub fn new_shared(webhook: WebhookReceiver) -> Arc<Self> {
        Arc::new(Self::new(webhook))
    }
}

impl LocalInjectDelivery for BroadcastInjectDelivery {
    fn deliver_inject(&self, _hashed_world: &str, packet: PeerPresenceInjectPacket) {
        let quic_packet = QuicNetworkPacket {
            packet_type: PacketType::PeerPresenceInject,
            owner: None,
            data: QuicNetworkPacketData::PeerPresenceInject(packet),
        };
        let webhook = self.webhook.clone();
        tokio::spawn(async move {
            if let Err(e) = webhook.send_packet(quic_packet).await {
                tracing::warn!("relay presence inject broadcast failed: {}", e);
            }
        });
    }

    fn deliver_announce(&self, packet: PeerAnnounceInjectPacket) {
        let quic_packet = QuicNetworkPacket {
            packet_type: PacketType::PeerAnnounceInject,
            owner: None,
            data: QuicNetworkPacketData::PeerAnnounceInject(packet),
        };
        let webhook = self.webhook.clone();
        tokio::spawn(async move {
            if let Err(e) = webhook.send_packet(quic_packet).await {
                tracing::warn!("relay announce broadcast failed: {}", e);
            }
        });
    }
}
