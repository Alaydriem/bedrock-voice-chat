use std::sync::Arc;
use std::time::{Duration, Instant};

use common::structs::packet::PeerAnnounceInjectPacket;

use super::pending_announce::PendingAnnounce;

pub struct AnnounceInjector {
    tx: flume::Sender<PendingAnnounce>,
    rx: flume::Receiver<PendingAnnounce>,
}

impl AnnounceInjector {
    pub fn new() -> Self {
        let (tx, rx) = flume::unbounded();
        Self { tx, rx }
    }

    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    pub fn handle_inject(&self, packet: &PeerAnnounceInjectPacket) {
        let deadline = Instant::now() + Duration::from_millis(u64::from(packet.ttl_ms));
        let _ = self.tx.try_send(PendingAnnounce {
            endpoint: packet.endpoint.clone(),
            deadline,
        });
    }

    pub fn receiver(&self) -> flume::Receiver<PendingAnnounce> {
        self.rx.clone()
    }
}

impl Default for AnnounceInjector {
    fn default() -> Self {
        Self::new()
    }
}
