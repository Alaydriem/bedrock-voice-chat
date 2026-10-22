use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// `protected-group:join-started`: a join request began from outside the window — the game or a
/// controller — so the webview opens the join dialog for `channel_id`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ProtectedGroupJoinStartedNotice {
    pub channel_id: String,
}
