use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// `protected-group:joined` and `protected-group:epoch`: the epoch and the code members compare.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ProtectedGroupEpochNotice {
    #[ts(type = "number")]
    pub epoch: u64,
    pub safety_code: String,
}
