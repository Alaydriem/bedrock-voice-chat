use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Whether the server relays in-game chat.
///
/// A server that predates this field sends nothing and reads as enabled, so an older
/// deployment never appears to have turned chat off.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ApiConfigChat {
    pub enabled: bool,
}

impl Default for ApiConfigChat {
    fn default() -> Self {
        Self { enabled: true }
    }
}
