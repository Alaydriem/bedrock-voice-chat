use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::structs::metrics::LinkDiagnosticsSnapshot;

// The WebSocket push envelope.
//
// The command protocol's `ResponseData` is `#[serde(untagged)]`, so a consumer cannot tell a
// pushed metrics frame from a pushed state frame by shape alone. Rather than add a variant
// there and inherit that ambiguity, the metrics stream carries its own discriminant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct MetricsPush {
    #[serde(rename = "type")]
    pub kind: String,
    pub data: LinkDiagnosticsSnapshot,
}

impl MetricsPush {
    pub const KIND: &'static str = "metrics";

    pub fn new(data: LinkDiagnosticsSnapshot) -> Self {
        Self {
            kind: Self::KIND.to_string(),
            data,
        }
    }
}
