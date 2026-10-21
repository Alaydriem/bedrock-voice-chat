use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// One joiner's card changed: `protected-group:check-failed`, `protected-group:request-ended`
/// or `protected-group:admitted`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ProtectedGroupJoinerNotice {
    #[ts(type = "number")]
    pub joiner_socket: u64,
}
