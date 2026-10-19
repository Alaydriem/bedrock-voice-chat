use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// `protected-group:lost`: why this client left the group. One of `wrong_password`,
/// `no_admitter`, `admitter_unreachable`, `socket_closed`, `protocol_failure`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ProtectedGroupLostNotice {
    pub reason: String,
}
