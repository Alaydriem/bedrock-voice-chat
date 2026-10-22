use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// `protected-group:checking`: another member is checking this joiner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ProtectedGroupCheckingNotice {
    #[ts(type = "number")]
    pub joiner_socket: u64,
    pub member_name: String,
}
