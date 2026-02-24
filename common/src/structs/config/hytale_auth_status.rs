use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Status of a Hytale device flow authentication
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum HytaleAuthStatus {
    Pending,
    Success,
    Expired,
    Denied,
    Error,
}
