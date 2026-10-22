use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// `protected-group:request`: someone asks to join this member's group. `name` is the identity
/// the server gave for the joiner's socket: a hint, not proof.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ProtectedGroupRequestNotice {
    #[ts(type = "number")]
    pub joiner_socket: u64,
    pub name: String,
}
