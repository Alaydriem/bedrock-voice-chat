pub mod config_error;
pub mod table;

pub use config_error::GrantConfigError;
pub use table::GrantTable;

use std::collections::HashSet;

use common::structs::relay::Capability;
use iroh::EndpointAddr;

// What one declared peer may do.
//
// `worlds` is a filter rather than a grant of worlds. It is empty for the
// ordinary block that names only a peer link, and an empty filter narrows
// nothing — the peer's own declaration stands.
#[derive(Debug, Clone)]
pub struct Grant {
    label: String,
    addr: EndpointAddr,
    worlds: Vec<String>,
    capabilities: HashSet<Capability>,
}

impl Grant {
    pub fn new(
        label: String,
        addr: EndpointAddr,
        worlds: Vec<String>,
        capabilities: HashSet<Capability>,
    ) -> Self {
        Self {
            label,
            addr,
            worlds,
            capabilities,
        }
    }

    // The `peer "<label>"` block this grant came from. Carried so a log line can
    // name what an operator wrote rather than the key they never typed.
    pub fn label(&self) -> &str {
        &self.label
    }

    // Where this peer can be reached, for the day this server dials rather than
    // waits. Carried because the peer link already contains it.
    pub fn addr(&self) -> &EndpointAddr {
        &self.addr
    }

    pub fn covers_world(&self, world: &str) -> bool {
        self.worlds.is_empty() || self.worlds.iter().any(|w| w == world)
    }

    // The declaration, less anything the filter excludes. Intersection only: a
    // filter naming a world the peer does not host adds nothing, because the peer
    // is the only side that knows what it hosts.
    pub fn narrow(&self, declared: &[String]) -> Vec<String> {
        declared
            .iter()
            .filter(|world| self.covers_world(world))
            .cloned()
            .collect()
    }

    pub fn allows(&self, capability: Capability) -> bool {
        self.capabilities.contains(&capability)
    }

    pub fn worlds(&self) -> &[String] {
        &self.worlds
    }

    // A HashSet has no order, and this value crosses a wire whose encoding is
    // pinned by test. Sorted by tag so the same grant always encodes the same.
    pub fn capabilities(&self) -> Vec<Capability> {
        let mut out: Vec<Capability> = self.capabilities.iter().copied().collect();
        out.sort_by_key(|c| c.as_str());
        out
    }
}
