use std::sync::Arc;
use std::time::Duration;

use common::structs::packet::PeerAnnounceInjectPacket;
use common::structs::relay::RelayEndpoint;

use super::orchestrator::LocalInjectDelivery;
use super::peer::table::PeerTable;

const DEFAULT_ANNOUNCE_INTERVAL_SECS: u64 = 60;

// How long an injected announce stays redeemable in the realm before the next
// cycle refreshes it. Matched to the announce cadence with headroom.
const ANNOUNCE_TTL_MS: u32 = 120_000;

// Seam for "which relay worlds is this server currently hosting active clients
// in". Backed by the live player cache; kept lock-light so the task never blocks
// the audio path.
pub trait ActiveWorldsSource: Send + Sync {
    fn active_worlds(&self) -> Vec<String>;
}

// Adapts a closure into an `ActiveWorldsSource` so callers that already hold an
// `Arc` over a cache can supply worlds without a bespoke type.
pub struct FnActiveWorldsSource<F: Fn() -> Vec<String> + Send + Sync>(pub F);

impl<F: Fn() -> Vec<String> + Send + Sync> ActiveWorldsSource for FnActiveWorldsSource<F> {
    fn active_worlds(&self) -> Vec<String> {
        (self.0)()
    }
}

// Periodically injects THIS server's advertised endpoint into every realm it
// hosts active clients in, as a suppressed `!bvca` chat. Peers observe it and
// populate their peer tables — the decentralized replacement for relay register.
// A dedicated `tokio::spawn` task; never on the QUIC receive path.
pub struct RelayAnnounceTask {
    inject: Arc<dyn LocalInjectDelivery>,
    peer_table: Arc<PeerTable>,
    self_endpoint: RelayEndpoint,
    worlds: Arc<dyn ActiveWorldsSource>,
    interval: Duration,
}

impl RelayAnnounceTask {
    pub fn new(
        inject: Arc<dyn LocalInjectDelivery>,
        peer_table: Arc<PeerTable>,
        self_endpoint: RelayEndpoint,
        worlds: Arc<dyn ActiveWorldsSource>,
    ) -> Self {
        Self::new_with_interval(
            inject,
            peer_table,
            self_endpoint,
            worlds,
            Duration::from_secs(DEFAULT_ANNOUNCE_INTERVAL_SECS),
        )
    }

    pub fn new_with_interval(
        inject: Arc<dyn LocalInjectDelivery>,
        peer_table: Arc<PeerTable>,
        self_endpoint: RelayEndpoint,
        worlds: Arc<dyn ActiveWorldsSource>,
        interval: Duration,
    ) -> Self {
        Self {
            inject,
            peer_table,
            self_endpoint,
            worlds,
            interval,
        }
    }

    fn endpoint_key(&self) -> String {
        format!("{}:{}", self.self_endpoint.host, self.self_endpoint.port)
    }

    // One announce cycle: refresh the active-world set the offer path reads, then
    // broadcast one self-announce per active world. With no active worlds nothing
    // is injected.
    pub async fn tick(&self) {
        let active = self.worlds.active_worlds();
        self.peer_table.set_active_worlds(active.clone());
        if active.is_empty() {
            return;
        }
        let endpoint = self.endpoint_key();
        for _world in &active {
            self.inject.deliver_announce(PeerAnnounceInjectPacket {
                endpoint: endpoint.clone(),
                ttl_ms: ANNOUNCE_TTL_MS,
            });
        }
    }

    pub async fn run(self) {
        let mut ticker = tokio::time::interval(self.interval);
        loop {
            ticker.tick().await;
            self.tick().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::structs::packet::PeerAnnounceInjectPacket;
    use std::sync::Mutex as StdMutex;

    struct CaptureInject {
        announced: StdMutex<Vec<String>>,
    }
    impl LocalInjectDelivery for CaptureInject {
        fn deliver_inject(
            &self,
            _hashed_world: &str,
            _packet: common::structs::packet::PeerPresenceInjectPacket,
        ) {
        }
        fn deliver_announce(&self, packet: PeerAnnounceInjectPacket) {
            self.announced.lock().unwrap().push(packet.endpoint);
        }
    }

    #[tokio::test]
    async fn tick_announces_self_endpoint_when_a_world_is_active() {
        let inject = Arc::new(CaptureInject {
            announced: StdMutex::new(Vec::new()),
        });
        let table = PeerTable::new_shared();
        let worlds: Arc<dyn ActiveWorldsSource> =
            Arc::new(FnActiveWorldsSource(|| vec!["W".to_string()]));
        let task = RelayAnnounceTask::new(
            inject.clone(),
            table.clone(),
            RelayEndpoint {
                host: "me".into(),
                port: 443,
                primary: false,
            },
            worlds,
        );
        task.tick().await;
        assert_eq!(
            inject.announced.lock().unwrap().clone(),
            vec!["me:443".to_string()]
        );
        assert_eq!(table.active_worlds(), vec!["W".to_string()]);
    }

    #[tokio::test]
    async fn tick_with_no_active_world_announces_nothing() {
        let inject = Arc::new(CaptureInject {
            announced: StdMutex::new(Vec::new()),
        });
        let table = PeerTable::new_shared();
        let worlds: Arc<dyn ActiveWorldsSource> = Arc::new(FnActiveWorldsSource(Vec::new));
        let task = RelayAnnounceTask::new(
            inject.clone(),
            table,
            RelayEndpoint {
                host: "me".into(),
                port: 443,
                primary: false,
            },
            worlds,
        );
        task.tick().await;
        assert!(inject.announced.lock().unwrap().is_empty());
    }
}
