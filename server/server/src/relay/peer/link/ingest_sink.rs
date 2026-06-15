use std::sync::Arc;

use common::structs::packet::QuicNetworkPacket;

use crate::stream::quic::WebhookReceiver;

// Where relayed inbound packets are published. The same entry point local
// clients' packets reach (the webhook/broadcast loop in `stream::quic`), so
// relayed audio flows through the identical `cache_manager.process_packet` +
// `route_audio_frame`/`broadcast_to_all` pipeline, updating only the ephemeral
// `player_cache` for proximity gating. It does not touch `PlayerRegistrarService`,
// so relayed players create no DB/user record.
#[async_trait::async_trait]
pub trait RelayIngestSink: Send + Sync {
    async fn publish(&self, packet: QuicNetworkPacket);
}

// Endpoint-scoped GATED ingest for an established peer link. Routes inbound
// datagrams through the presence-proof gate (`PeerManager::ingest`): an unproven
// peer's AUDIO/position packets are dropped fail-closed and only presence-control
// packets are allowed pre-proof. Both peer-link read pumps — acceptor and dialer
// — publish through this so the gate applies in BOTH directions. The endpoint is
// bound at construction so the read pump need not know its own peer identity.
#[async_trait::async_trait]
pub trait GatedPeerIngest: Send + Sync {
    async fn ingest_from_peer(&self, packet: QuicNetworkPacket);
}

// Production sink: forwards into the existing webhook/broadcast loop.
pub struct WebhookIngestSink {
    receiver: WebhookReceiver,
}

impl WebhookIngestSink {
    pub fn new(receiver: WebhookReceiver) -> Self {
        Self { receiver }
    }

    pub fn new_shared(receiver: WebhookReceiver) -> Arc<Self> {
        Arc::new(Self::new(receiver))
    }
}

#[async_trait::async_trait]
impl RelayIngestSink for WebhookIngestSink {
    async fn publish(&self, packet: QuicNetworkPacket) {
        if let Err(e) = self.receiver.send_packet(packet).await {
            tracing::warn!("relay ingest publish failed: {}", e);
        }
    }
}
