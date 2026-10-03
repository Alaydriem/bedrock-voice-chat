use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Whether the app may point the idle ring at players beyond voice range.
///
/// A server that predates this field sends nothing and reads as enabled, so an older
/// deployment keeps the ring it always had.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ApiConfigRadar {
    pub enabled: bool,
}

impl Default for ApiConfigRadar {
    fn default() -> Self {
        Self { enabled: true }
    }
}
