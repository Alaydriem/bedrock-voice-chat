use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// `protected-group:lost`: why this client left the group, or never got in. One of `refused`,
/// `mismatch`, `expired`, `no_admitter`, `socket_closed`, `protocol_failure`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ProtectedGroupLostNotice {
    pub reason: String,
}
