use std::collections::{HashMap, HashSet};
use std::sync::RwLock;

use bvc_relay::node::PeerTicket;
use bvc_relay::peer::{PeerAuthority, PeerScope, RedeemResult};
use common::curia;
use common::structs::relay::Capability;
use common::structs::relay::wire::control::RefuseReason;
use iroh::PublicKey;
use sea_orm::DatabaseConnection;

use crate::config::PeerConfig;
use crate::services::pairing_service::RedeemOutcome;

use super::Grant;
use super::config_error::GrantConfigError;

// Every peer this server will speak to, and what each may do.
//
// Two sources with a fixed precedence. `declared` comes from `config.hcl` and never
// changes for the life of the process. `paired` comes from the database and gains an
// entry when a bridge redeems a pairing code. A node absent from both is refused at
// connect, which is what keeps the pre-authorization window short.
//
// Lookups are synchronous because `authorize` answers on the connect path and
// `may_carry` on the packet path. The paired half is therefore read once at startup and
// updated in memory by the redemption that wrote it, rather than queried per call.
pub struct GrantTable {
    declared: HashMap<PublicKey, Grant>,
    paired: RwLock<HashMap<PublicKey, Grant>>,
    // `None` for a table built from configuration alone, which has no store to redeem
    // against. Such a table refuses every code rather than pretending to check one.
    conn: Option<DatabaseConnection>,
}

impl GrantTable {
    pub fn from_config(peers: &HashMap<String, PeerConfig>) -> Result<Self, GrantConfigError> {
        Ok(Self {
            declared: Self::declared_from_config(peers)?,
            paired: RwLock::new(HashMap::new()),
            conn: None,
        })
    }

    /// Every peer this server will speak to: those declared in `config.hcl`, and those
    /// that redeemed a pairing code.
    ///
    /// Paired rows are read once here and held in memory. Redemption inserts into this
    /// cache as well as the table, so the two agree for the life of the process; a grant
    /// revoked directly in the database is not seen until the next start.
    pub async fn from_config_and_db(
        peers: &HashMap<String, PeerConfig>,
        conn: &DatabaseConnection,
    ) -> Result<Self, GrantConfigError> {
        let declared = Self::declared_from_config(peers)?;

        let rows = crate::services::PairingService::paired(conn)
            .await
            .map_err(|e| GrantConfigError::PairedRow {
                reason: e.to_string(),
            })?;

        let mut paired = HashMap::new();
        for row in rows {
            let node = Self::parse_node(&row.node_id)?;
            paired.insert(node, Self::grant_from_row(&row)?);
        }

        Ok(Self {
            declared,
            paired: RwLock::new(paired),
            conn: Some(conn.clone()),
        })
    }

    /// Adds a grant a redemption just wrote, so the next connection is authorized without
    /// a restart.
    pub fn remember(&self, node: PublicKey, grant: Grant) {
        if let Ok(mut paired) = self.paired.write() {
            paired.insert(node, grant);
        }
    }

    /// Drops every grant carrying this label, so a revocation takes effect without a
    /// restart. Returns how many were dropped.
    pub fn forget(&self, label: &str) -> usize {
        let Ok(mut paired) = self.paired.write() else {
            return 0;
        };

        let before = paired.len();
        paired.retain(|_, grant| grant.label() != label);

        before - paired.len()
    }

    pub fn connection(&self) -> Option<&DatabaseConnection> {
        self.conn.as_ref()
    }

    // Declared outranks paired: config is what an operator wrote deliberately, and a
    // bridge that later pairs must not silently replace it.
    pub fn grant_for(&self, node: &PublicKey) -> Option<Grant> {
        if let Some(grant) = self.declared.get(node) {
            return Some(grant.clone());
        }

        self.paired.read().ok()?.get(node).cloned()
    }

    // Every world named by a `worlds` filter, against the block that named it.
    //
    // Declared blocks only: this reports what an operator wrote and might have got wrong.
    // A paired grant carries no filter, so it has nothing to report.
    //
    // Sorted, because a `HashMap` iteration order would reorder the warnings between runs
    // of a server that changed nothing.
    pub fn configured_worlds(&self) -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> = self
            .declared
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

    // The question the ingest path asks per packet: may this node speak for this world?
    // Fail-closed on an unknown node, an uncovered world, or a missing capability.
    pub fn may_carry(&self, node: &PublicKey, world: &str) -> bool {
        self.grant_for(node).is_some_and(|grant| {
            grant.allows(Capability::CarrySpeakers) && grant.covers_world(world)
        })
    }

    pub fn len(&self) -> usize {
        let paired = self.paired.read().map(|p| p.len()).unwrap_or(0);

        self.declared.len() + paired
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn declared_from_config(
        peers: &HashMap<String, PeerConfig>,
    ) -> Result<HashMap<PublicKey, Grant>, GrantConfigError> {
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

        Ok(grants)
    }

    fn parse_node(hex_value: &str) -> Result<PublicKey, GrantConfigError> {
        let bytes: [u8; 32] = hex::decode(hex_value)
            .ok()
            .and_then(|b| b.try_into().ok())
            .ok_or_else(|| GrantConfigError::PairedRow {
                reason: format!("{hex_value:?} is not a node key"),
            })?;

        PublicKey::from_bytes(&bytes).map_err(|e| GrantConfigError::PairedRow {
            reason: e.to_string(),
        })
    }

    fn grant_from_row(row: &entity::peer_grant::Model) -> Result<Grant, GrantConfigError> {
        let worlds: Vec<String> = row
            .worlds
            .split(',')
            .filter(|w| !w.is_empty())
            .map(str::to_string)
            .collect();

        let mut capabilities = HashSet::new();
        for tag in row.capabilities.split(',').filter(|c| !c.is_empty()) {
            let capability =
                Capability::from_tag(tag).ok_or_else(|| GrantConfigError::PairedRow {
                    reason: format!("unknown capability {tag:?}"),
                })?;
            capabilities.insert(capability);
        }

        Ok(Grant::new(
            row.label.clone(),
            iroh::EndpointAddr::new(Self::parse_node(&row.node_id)?),
            worlds,
            capabilities,
        ))
    }
}

// The transport asks who may peer; this table is where a server keeps the answer.
#[async_trait::async_trait]
impl PeerAuthority for GrantTable {
    fn authorize(&self, node: &PublicKey, declared: &[String]) -> Option<PeerScope> {
        let grant = self.grant_for(node)?;

        Some(PeerScope {
            worlds: grant.narrow(declared),
            capabilities: grant.capabilities(),
        })
    }

    async fn redeem(&self, node: &PublicKey, code: &str, declared: &[String]) -> RedeemResult {
        // A table built from configuration alone has no store to check a code against.
        // Refusing is the only honest answer; pretending to check one would admit a peer
        // no operator authorized.
        let Some(conn) = self.conn.as_ref() else {
            return RedeemResult::Refused(RefuseReason::UnknownCode);
        };

        let outcome = match crate::services::PairingService::redeem(
            conn,
            &hex::encode(node.as_bytes()),
            code,
            declared,
        )
        .await
        {
            Ok(outcome) => outcome,
            Err(e) => {
                curia::warn!(format!(
                    "a pairing redemption could not reach the store: {e}"
                ));
                return RedeemResult::Refused(RefuseReason::UnknownCode);
            }
        };

        match outcome {
            RedeemOutcome::Paired {
                label,
                worlds,
                capabilities,
            } => {
                self.remember(
                    *node,
                    Grant::new(
                        label,
                        iroh::EndpointAddr::new(*node),
                        // Empty: a paired grant narrows nothing, so the peer's own
                        // declaration stands.
                        Vec::new(),
                        capabilities.iter().copied().collect(),
                    ),
                );

                RedeemResult::Granted(PeerScope {
                    worlds,
                    capabilities,
                })
            }
            // The cache entry was written when this node first paired and has not changed.
            RedeemOutcome::AlreadyPaired {
                worlds,
                capabilities,
                ..
            } => RedeemResult::Granted(PeerScope {
                worlds,
                capabilities,
            }),
            RedeemOutcome::Unknown => RedeemResult::Refused(RefuseReason::UnknownCode),
            RedeemOutcome::Spent => RedeemResult::Refused(RefuseReason::CodeSpent),
            RedeemOutcome::Expired => RedeemResult::Refused(RefuseReason::CodeExpired),
        }
    }
}
