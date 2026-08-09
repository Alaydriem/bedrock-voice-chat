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
/// Carries no address. A controller picks a name from this list and quotes the id back; the
/// client is what knows where that world is. `kind` is derivable from the id's prefix and is
/// named anyway so a picker can group without parsing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ConnectTarget {
    pub id: String,
    pub name: String,
    pub kind: ConnectTargetKind,
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
}
