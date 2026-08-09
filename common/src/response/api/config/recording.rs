use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Whether the server permits clients to record voice sessions.
///
/// A server that predates this field sends nothing and reads as permitted, so an
/// older deployment never appears to have turned recording off.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ApiConfigRecording {
    pub enabled: bool,
}

impl Default for ApiConfigRecording {
    fn default() -> Self {
        Self { enabled: true }
    }
}
