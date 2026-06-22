use std::sync::Arc;
use std::time::{Duration, Instant};

use common::structs::packet::PeerPresenceInjectPacket;

use super::pending_inject::PendingInject;

pub struct PresenceInjector {
    tx: flume::Sender<PendingInject>,
    rx: flume::Receiver<PendingInject>,
}

impl PresenceInjector {
    pub fn new() -> Self {
        let (tx, rx) = flume::unbounded();
        Self { tx, rx }
    }

    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    pub fn handle_inject(&self, packet: &PeerPresenceInjectPacket) {
        let deadline = Instant::now() + Duration::from_millis(u64::from(packet.ttl_ms));
        let _ = self.tx.try_send(PendingInject {
            token: packet.token.clone(),
            deadline,
        });
    }

    pub fn receiver(&self) -> flume::Receiver<PendingInject> {
        self.rx.clone()
    }
}

impl Default for PresenceInjector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enqueues_inject_with_token() {
        let injector = PresenceInjector::new();
        let rx = injector.receiver();
        injector.handle_inject(&PeerPresenceInjectPacket {
            token: "tok".to_string(),
            ttl_ms: 5000,
        });
        let pending = rx.try_recv().expect("inject should enqueue");
        assert_eq!(pending.token, "tok");
        assert!(!pending.is_expired(Instant::now()));
    }

    #[test]
    fn zero_ttl_is_immediately_expired() {
        let injector = PresenceInjector::new();
        let rx = injector.receiver();
        injector.handle_inject(&PeerPresenceInjectPacket {
            token: "tok".to_string(),
            ttl_ms: 0,
        });
        let pending = rx.try_recv().expect("inject should enqueue");
        assert!(pending.is_expired(Instant::now()));
    }
}
