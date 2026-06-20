use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use dashmap::DashMap;

use common::structs::relay::RelayEndpoint;

#[derive(Clone)]
pub struct PeerTable {
    peers: Arc<DashMap<String, HashMap<String, (RelayEndpoint, Instant)>>>,
    active_worlds: Arc<RwLock<HashSet<String>>>,
}

impl PeerTable {
    pub fn new() -> Self {
        Self {
            peers: Arc::new(DashMap::new()),
            active_worlds: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    fn endpoint_key(ep: &RelayEndpoint) -> String {
        format!("{}:{}", ep.host, ep.port)
    }

    // Records a peer endpoint observed (via a realm `!bvca` announce) as live for
    // `hashed_world` until `now + ttl`. Re-observing the same endpoint refreshes
    // its expiry rather than duplicating it.
    pub fn observe_peer(
        &self,
        hashed_world: &str,
        endpoint: RelayEndpoint,
        now: Instant,
        ttl: Duration,
    ) {
        let key = Self::endpoint_key(&endpoint);
        let mut world = self.peers.entry(hashed_world.to_string()).or_default();
        world.insert(key, (endpoint, now + ttl));
    }

    // Test/compat helper: overwrite a world's peer set with a far-future expiry.
    pub fn set_world_peers(&self, hashed_world: &str, peers: Vec<RelayEndpoint>) {
        let now = Instant::now();
        let ttl = Duration::from_secs(3600);
        self.peers.remove(hashed_world);
        for p in peers {
            self.observe_peer(hashed_world, p, now, ttl);
        }
    }

    pub fn peers_for_world(&self, hashed_world: &str) -> Vec<RelayEndpoint> {
        let now = Instant::now();
        self.peers
            .get(hashed_world)
            .map(|world| {
                world
                    .iter()
                    .filter(|(_, (_, expires))| *expires > now)
                    .map(|(_, (ep, _))| ep.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    // Drops expired entries so a peer that stopped announcing is forgotten.
    pub fn sweep_expired(&self, now: Instant) {
        for mut world in self.peers.iter_mut() {
            world.retain(|_, (_, expires)| *expires > now);
        }
        self.peers.retain(|_, world| !world.is_empty());
    }

    pub fn set_active_worlds(&self, worlds: Vec<String>) {
        let set: HashSet<String> = worlds.into_iter().collect();
        let mut guard = self
            .active_worlds
            .write()
            .expect("active_worlds lock poisoned");
        *guard = set;
    }

    pub fn active_worlds(&self) -> Vec<String> {
        self.active_worlds
            .read()
            .expect("active_worlds lock poisoned")
            .iter()
            .cloned()
            .collect()
    }

    pub fn is_world_active(&self, hashed_world: &str) -> bool {
        self.active_worlds
            .read()
            .expect("active_worlds lock poisoned")
            .contains(hashed_world)
    }
}

impl Default for PeerTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ep(host: &str, port: u16) -> RelayEndpoint {
        RelayEndpoint {
            host: host.to_string(),
            port,
            primary: false,
        }
    }

    fn sorted_keys(peers: &[RelayEndpoint]) -> Vec<String> {
        let mut keys: Vec<String> = peers.iter().map(PeerTable::endpoint_key).collect();
        keys.sort();
        keys
    }

    #[test]
    fn set_and_read_world_peers() {
        let table = PeerTable::new();
        let a = ep("a", 1);
        let b = ep("b", 2);
        table.set_world_peers("hW", vec![a.clone(), b.clone()]);
        assert_eq!(
            sorted_keys(&table.peers_for_world("hW")),
            vec!["a:1".to_string(), "b:2".to_string()]
        );
    }

    #[test]
    fn unknown_world_has_no_peers() {
        let table = PeerTable::new();
        assert!(table.peers_for_world("missing").is_empty());
    }

    #[test]
    fn active_worlds_round_trip() {
        let table = PeerTable::new();
        table.set_active_worlds(vec!["w1".into(), "w2".into()]);
        let mut got = table.active_worlds();
        got.sort();
        assert_eq!(got, vec!["w1".to_string(), "w2".to_string()]);
        assert!(table.is_world_active("w1"));
        assert!(!table.is_world_active("w3"));
    }

    #[test]
    fn set_world_peers_overwrites() {
        let table = PeerTable::new();
        table.set_world_peers("hW", vec![ep("a", 1)]);
        table.set_world_peers("hW", vec![ep("b", 2)]);
        assert_eq!(table.peers_for_world("hW"), vec![ep("b", 2)]);
    }

    #[test]
    fn observed_peer_expires_after_ttl() {
        let table = PeerTable::new();
        let t0 = Instant::now();
        table.observe_peer("W", ep("b", 2), t0, Duration::from_secs(300));
        assert_eq!(table.peers_for_world("W"), vec![ep("b", 2)]);
        // Re-observing the same endpoint refreshes, never duplicates.
        table.observe_peer(
            "W",
            ep("b", 2),
            t0 + Duration::from_secs(100),
            Duration::from_secs(300),
        );
        assert_eq!(table.peers_for_world("W").len(), 1);
        // Past expiry the sweep forgets it.
        table.sweep_expired(t0 + Duration::from_secs(401));
        assert!(table.peers_for_world("W").is_empty());
    }
}
