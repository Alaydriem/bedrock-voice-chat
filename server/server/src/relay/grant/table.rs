use std::collections::{HashMap, HashSet};

use bvc_relay::node::PeerTicket;
use bvc_relay::peer::{PeerAuthority, PeerScope};
use common::structs::relay::Capability;
use iroh::PublicKey;

use crate::config::PeerConfig;

use super::config_error::GrantConfigError;
use super::Grant;

// Every peer this server will speak to, and what each may do.
//
// Built once at startup from `config.hcl` and never mutated: peering is declared,
// so there is no runtime path that grants anything. A node absent here is refused
// at connect, which is what keeps the pre-authorization window short.
pub struct GrantTable {
    grants: HashMap<PublicKey, Grant>,
}

impl GrantTable {
    pub fn from_config(peers: &HashMap<String, PeerConfig>) -> Result<Self, GrantConfigError> {
        let mut grants = HashMap::new();
        let mut labels: HashMap<PublicKey, String> = HashMap::new();

        for (label, cfg) in peers {
            let addr = PeerTicket::parse(&cfg.peerlink).map_err(|e| GrantConfigError::PeerLink {
                label: label.clone(),
                reason: e.to_string(),
            })?;
            let node = addr.id;

            let mut capabilities = HashSet::new();
            for raw in &cfg.capabilities {
                let capability =
                    Capability::from_tag(raw).ok_or_else(|| GrantConfigError::Capability {
                        label: label.clone(),
                        value: raw.clone(),
                    })?;
                capabilities.insert(capability);
            }

            if let Some(first) = labels.get(&node) {
                return Err(GrantConfigError::DuplicateNode {
                    first: first.clone(),
                    second: label.clone(),
                });
            }
            labels.insert(node, label.clone());

            grants.insert(
                node,
                Grant::new(label.clone(), addr, cfg.worlds.clone(), capabilities),
            );
        }

        Ok(Self { grants })
    }

    pub fn grant_for(&self, node: &PublicKey) -> Option<&Grant> {
        self.grants.get(node)
    }

    // Every world named by a `worlds` filter, against the block that named it.
    //
    // Sorted, because a `HashMap` iteration order would reorder the warnings
    // between runs of a server that changed nothing.
    pub fn configured_worlds(&self) -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> = self
            .grants
            .values()
            .flat_map(|grant| {
                grant
                    .worlds()
                    .iter()
                    .map(|world| (grant.label().to_string(), world.clone()))
            })
            .collect();

        out.sort();
        out
    }

    // The question the ingest path asks per packet: may this node speak for this
    // world? Fail-closed on an unknown node, an uncovered world, or a missing
    // capability.
    pub fn may_carry(&self, node: &PublicKey, world: &str) -> bool {
        self.grants.get(node).is_some_and(|grant| {
            grant.allows(Capability::CarrySpeakers) && grant.covers_world(world)
        })
    }

    pub fn len(&self) -> usize {
        self.grants.len()
    }

    pub fn is_empty(&self) -> bool {
        self.grants.is_empty()
    }
}

// The transport asks who may peer; this table is where a server keeps the answer.
// Config is the whole of it — there is no runtime path that grants anything.
impl PeerAuthority for GrantTable {
    fn authorize(&self, node: &PublicKey, declared: &[String]) -> Option<PeerScope> {
        let grant = self.grants.get(node)?;

        Some(PeerScope {
            worlds: grant.narrow(declared),
            capabilities: grant.capabilities(),
        })
    }
}
