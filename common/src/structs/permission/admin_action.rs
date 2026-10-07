use serde::{Deserialize, Serialize};

/// A change to a player's `admin` permission.
///
/// `Revoke` removes the override, so the configured default applies again. `Deny` records an
/// explicit refusal that outranks the default.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AdminAction {
    Grant,
    Revoke,
    Deny,
}
