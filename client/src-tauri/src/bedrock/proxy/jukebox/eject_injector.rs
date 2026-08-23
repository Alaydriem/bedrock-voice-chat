use std::sync::Arc;
use std::time::Duration;

use common::structs::packet::{BedrockEvent, BedrockEventDirection, BedrockEventPacket};
use moka::sync::Cache;

use super::pending_eject::PendingEject;

const SEEN_TTL: Duration = Duration::from_secs(5);
const SEEN_CAPACITY: u64 = 256;

pub struct JukeboxEjectInjector {
    tx: flume::Sender<PendingEject>,
    rx: flume::Receiver<PendingEject>,
    seen: Cache<String, ()>,
}

impl JukeboxEjectInjector {
    pub fn new() -> Self {
        let (tx, rx) = flume::unbounded();
        Self {
            tx,
            rx,
            seen: Cache::builder()
                .time_to_live(SEEN_TTL)
                .max_capacity(SEEN_CAPACITY)
                .build(),
        }
    }

    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    pub fn handle_packet(&self, packet: &BedrockEventPacket) {
        match packet.direction {
            BedrockEventDirection::ClientBound => {}
            _ => return,
        }

        let (event_id, block_pos) = match &packet.event {
            BedrockEvent::JukeboxEjectAnnouncement {
                event_id,
                block_pos,
            } => (event_id.clone(), block_pos.clone()),
            _ => return,
        };

        if self.seen.get(&event_id).is_some() {
            log::debug!(
                "JukeboxEjectInjector: dropping duplicate announcement event_id={}",
                event_id
            );
            return;
        }
        self.seen.insert(event_id.clone(), ());

        log::info!(
            "JukeboxEjectInjector: enqueue eject event_id={} world={} pos=({},{},{})",
            event_id,
            packet.world_uuid,
            block_pos.x,
            block_pos.y,
            block_pos.z
        );
        let _ = self.tx.try_send(PendingEject {
            event_id,
            world_uuid: packet.world_uuid.clone(),
            block_pos,
        });
    }

    pub fn receiver(&self) -> flume::Receiver<PendingEject> {
        self.rx.clone()
    }
}

impl Default for JukeboxEjectInjector {
    fn default() -> Self {
        Self::new()
    }
}
