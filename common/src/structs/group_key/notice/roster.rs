use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// `protected-group:roster`: canonical identities from the group's own MLS state, never the
/// server's channel list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ProtectedGroupRosterNotice {
    pub members: Vec<String>,
}
