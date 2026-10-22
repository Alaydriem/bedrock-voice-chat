use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// `protected-group:waiting`: requests beyond the cards shown, for the "+N more" line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ProtectedGroupWaitingNotice {
    pub count: u32,
}
