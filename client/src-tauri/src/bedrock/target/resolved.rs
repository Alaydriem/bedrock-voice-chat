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
        ConnectTarget::new(self.id.clone(), self.name.clone(), self.kind())
    }

    pub fn to_active(&self) -> ActiveConnection {
        ActiveConnection::new(self.id.clone(), self.name.clone(), self.kind())
    }
}
