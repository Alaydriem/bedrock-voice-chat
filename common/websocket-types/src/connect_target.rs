use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ConnectTargetKind {
    Proxy,
    Realm,
}

/// One world a controller can ask the client to connect to.
///
/// `host` and `port` are the proxy's backend and are absent for a realm, whose address the
/// client resolves from Xbox Live at connect time. `protocol_version` carries the saved
/// entry's advertised Bedrock version so a scripted connect and a click advertise the same
/// thing; `None` means the proxy mirrors the real backend.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ConnectTarget {
    pub id: String,
    pub name: String,
    pub kind: ConnectTargetKind,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub protocol_version: Option<u32>,
}

impl ConnectTarget {
    /// The entry a `connect` names, matched by id.
    ///
    /// Never by list position: listing and connecting are two calls, and an entry added
    /// between them would otherwise shift the operator onto a different world than the one
    /// they read.
    pub fn find<'a>(targets: &'a [ConnectTarget], id: &str) -> Option<&'a ConnectTarget> {
        targets.iter().find(|target| target.id == id)
    }

    pub fn is_connectable(&self) -> bool {
        match self.kind {
            ConnectTargetKind::Proxy => self.host.is_some() && self.port.is_some(),
            ConnectTargetKind::Realm => true,
        }
    }
}
