use websocket_types::{ActiveConnection, ConnectTarget, ConnectTargetKind};

use super::ResolvedAddress;

/// A connectable world, with the address its wire form omits.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedTarget {
    pub id: String,
    pub name: String,
    pub address: ResolvedAddress,
}

impl ResolvedTarget {
    pub fn kind(&self) -> ConnectTargetKind {
        match self.address {
            ResolvedAddress::Proxy { .. } => ConnectTargetKind::Proxy,
            ResolvedAddress::Realm { .. } => ConnectTargetKind::Realm,
        }
    }

    pub fn to_wire(&self) -> ConnectTarget {
        ConnectTarget {
            id: self.id.clone(),
            name: self.name.clone(),
            kind: self.kind(),
        }
    }

    pub fn to_active(&self) -> ActiveConnection {
        ActiveConnection {
            id: self.id.clone(),
            name: self.name.clone(),
            kind: self.kind(),
        }
    }
}
