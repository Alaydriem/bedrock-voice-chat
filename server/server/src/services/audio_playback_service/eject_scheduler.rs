use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use common::Coordinate;
use common::structs::packet::{
    BedrockEvent, BedrockEventDirection, BedrockEventPacket, PacketType, QuicNetworkPacket,
    QuicNetworkPacketData,
};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::services::BedrockEventService;
use crate::stream::quic::WebhookReceiver;

pub struct EjectScheduler {
    pending: Mutex<HashMap<String, JoinHandle<()>>>,
    bedrock_event_service: Arc<BedrockEventService>,
    webhook_receiver: WebhookReceiver,
}

impl EjectScheduler {
    pub fn new(
        bedrock_event_service: Arc<BedrockEventService>,
        webhook_receiver: WebhookReceiver,
    ) -> Self {
        Self {
            pending: Mutex::new(HashMap::new()),
            bedrock_event_service,
            webhook_receiver,
        }
    }

    pub fn new_shared(
        bedrock_event_service: Arc<BedrockEventService>,
        webhook_receiver: WebhookReceiver,
    ) -> Arc<Self> {
        Arc::new(Self::new(bedrock_event_service, webhook_receiver))
    }

    pub async fn schedule(
        self: &Arc<Self>,
        event_id: String,
        world_uuid: String,
        block_pos: Coordinate,
        duration: Duration,
    ) {
        tracing::info!(
            event_id = %event_id,
            world_uuid = %world_uuid,
            duration_ms = duration.as_millis() as u64,
            "EjectScheduler: scheduling auto-eject"
        );
        let scheduler = Arc::clone(self);
        let event_id_for_task = event_id.clone();

        let handle = tokio::spawn(async move {
            tokio::time::sleep(duration).await;
            scheduler
                .fire(event_id_for_task, world_uuid, block_pos)
                .await;
        });

        let mut pending = self.pending.lock().await;
        if let Some(prev) = pending.insert(event_id, handle) {
            prev.abort();
        }
    }

    pub async fn cancel(&self, event_id: &str) {
        let mut pending = self.pending.lock().await;
        if let Some(handle) = pending.remove(event_id) {
            handle.abort();
        }
    }

    async fn fire(&self, event_id: String, world_uuid: String, block_pos: Coordinate) {
        tracing::info!(event_id = %event_id, "EjectScheduler: timer fired");
        {
            let mut pending = self.pending.lock().await;
            pending.remove(&event_id);
        }

        if self.bedrock_event_service.is_bds_healthy(&world_uuid).await {
            tracing::debug!(
                event_id = %event_id,
                world_uuid = %world_uuid,
                "EjectScheduler: skipping broadcast (BDS addon is healthy)"
            );
            return;
        }

        let announcement = BedrockEvent::JukeboxEjectAnnouncement {
            event_id: event_id.clone(),
            block_pos,
        };
        let bedrock_packet = BedrockEventPacket::with_direction(
            announcement,
            world_uuid.clone(),
            BedrockEventDirection::ClientBound,
        );
        let packet = QuicNetworkPacket {
            packet_type: PacketType::BedrockEvent,
            data: QuicNetworkPacketData::BedrockEvent(bedrock_packet),
                    // Not a server fan-out, so this envelope carries no sequence.
            ..Default::default()
        };

        if let Err(e) = self.webhook_receiver.send_packet(packet).await {
            tracing::warn!(
                event_id = %event_id,
                world_uuid = %world_uuid,
                error = %e,
                "EjectScheduler: failed to broadcast JukeboxEjectAnnouncement"
            );
        } else {
            tracing::debug!(
                event_id = %event_id,
                world_uuid = %world_uuid,
                "EjectScheduler: broadcast ClientBound JukeboxEjectAnnouncement"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn cancel_aborts_pending_task() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let webhook = WebhookReceiver::new(tx);
        let bes = build_bedrock_event_service(webhook.clone()).await;
        let scheduler = EjectScheduler::new_shared(bes, webhook);

        scheduler
            .schedule(
                "evt-1".to_string(),
                "world-x".to_string(),
                Coordinate {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                Duration::from_secs(60),
            )
            .await;
        scheduler.cancel("evt-1").await;

        let pending = scheduler.pending.lock().await;
        assert!(pending.is_empty());
    }

    async fn build_bedrock_event_service(webhook: WebhookReceiver) -> Arc<BedrockEventService> {
        let db = sea_orm::Database::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite");
        let conn = Arc::new(db);
        BedrockEventService::new_shared(
            Arc::new(crate::services::AudioPlaybackService::new(
                webhook.clone(),
                String::new(),
                tokio_util::sync::CancellationToken::new(),
                1,
            )),
            webhook,
            conn,
            30,
        )
    }
}
