use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::ServerCheckFailure;
use crate::response::ApiConfigResponse;

/// The sign-in screen's check of a server address.
///
/// A failure is an answer, not an error: the screen shows it. The command's own `Err` is kept
/// for faults that say nothing about the server.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "state")]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum ServerCheckReport {
    Reachable {
        config: ApiConfigResponse,
    },
    Failed {
        failure: ServerCheckFailure,
        detail: String,
    },
}
