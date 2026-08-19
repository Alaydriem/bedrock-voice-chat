use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// How an admin mutation ended, for a caller that has to say something different about
/// each case.
///
/// A typed answer rather than a status code flattened into an error string: the pane
/// distinguishes "you cannot ban yourself" from "the server is unreachable", and a string
/// comparison would break the moment the copy is translated. Modelled on
/// `CredentialVerdict`, which exists for the same reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum AdminActionOutcome {
    /// The server applied the change.
    Applied,
    /// Refused by one of the self-protection guards.
    Conflict,
    /// No such player, or no such override to clear.
    NotFound,
    /// The caller no longer holds `admin`.
    Forbidden,
    /// The server rejected the request itself: an unknown permission, a malformed gamertag.
    Invalid,
}
