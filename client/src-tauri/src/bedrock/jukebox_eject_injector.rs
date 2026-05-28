use std::sync::Arc;
use std::time::Duration;

use common::structs::packet::{BedrockEvent, BedrockEventDirection, BedrockEventPacket};
use moka::sync::Cache;

use crate::bedrock::pending_eject::PendingEject;

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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_announcement(event_id: &str, world_uuid: &str) -> BedrockEventPacket {
        BedrockEventPacket::with_direction(
            BedrockEvent::JukeboxEjectAnnouncement {
                event_id: event_id.to_string(),
                block_pos: Coordinate {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                },
            },
            world_uuid.to_string(),
            BedrockEventDirection::ClientBound,
        )
    }

    #[test]
    fn enqueues_first_arrival() {
        let injector = JukeboxEjectInjector::new();
        let rx = injector.receiver();
        injector.handle_packet(&make_announcement("e1", "world-x"));
        let job = rx.try_recv().expect("first arrival should enqueue");
        assert_eq!(job.event_id, "e1");
        assert_eq!(job.world_uuid, "world-x");
    }

    #[test]
    fn dedups_repeat_event_id() {
        let injector = JukeboxEjectInjector::new();
        let rx = injector.receiver();
        injector.handle_packet(&make_announcement("e1", "world-x"));
        injector.handle_packet(&make_announcement("e1", "world-x"));
        assert!(rx.try_recv().is_ok());
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn ignores_non_client_bound_direction() {
        let injector = JukeboxEjectInjector::new();
        let rx = injector.receiver();
        let mut pkt = make_announcement("e1", "world-x");
        pkt.direction = BedrockEventDirection::ServerBound;
        injector.handle_packet(&pkt);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn ignores_non_announcement_variants() {
        let injector = JukeboxEjectInjector::new();
        let rx = injector.receiver();
        let pkt = BedrockEventPacket::with_direction(
            BedrockEvent::JukeboxEject {
                event_id: "e1".to_string(),
                player_xuid: "p1".to_string(),
            },
            "world-x".to_string(),
            BedrockEventDirection::ClientBound,
        );
        injector.handle_packet(&pkt);
        assert!(rx.try_recv().is_err());
    }
}
