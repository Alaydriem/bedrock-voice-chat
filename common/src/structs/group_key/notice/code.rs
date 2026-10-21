use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// `protected-group:code`: a code to compare for one check. `role` is `joiner` or `member`;
/// `check_id` is 32 hex characters. The webview places a member's code only on the card whose
/// `joiner_socket` matches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ProtectedGroupCodeNotice {
    pub role: String,
    pub check_id: String,
    #[ts(type = "number")]
    pub joiner_socket: u64,
    pub peer_name: String,
    pub code: String,
}
