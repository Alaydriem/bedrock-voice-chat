use std::collections::HashSet;
use std::sync::{Arc, RwLock};

use dashmap::DashMap;

use common::structs::relay::RelayEndpoint;

#[derive(Clone)]
pub struct PeerTable {
    peers: Arc<DashMap<String, Vec<RelayEndpoint>>>,
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

    pub fn set_world_peers(&self, hashed_world: &str, peers: Vec<RelayEndpoint>) {
        self.peers.insert(hashed_world.to_string(), peers);
    }

    pub fn peers_for_world(&self, hashed_world: &str) -> Vec<RelayEndpoint> {
        self.peers
            .get(hashed_world)
            .map(|p| p.clone())
            .unwrap_or_default()
    }

    pub fn set_active_worlds(&self, worlds: Vec<String>) {
        let set: HashSet<String> = worlds.into_iter().collect();
        let mut guard = self.active_worlds.write().expect("active_worlds lock poisoned");
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

    #[test]
    fn set_and_read_world_peers() {
        let table = PeerTable::new();
        let a = ep("a", 1);
        let b = ep("b", 2);
        table.set_world_peers("hW", vec![a.clone(), b.clone()]);
        assert_eq!(table.peers_for_world("hW"), vec![a, b]);
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
}
