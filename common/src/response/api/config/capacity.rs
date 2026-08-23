use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// How many concurrent voice sessions this server admits, and how many it holds now.
///
/// A server that predates this field sends nothing and reads as unlimited, so an older
/// deployment never appears to be full.
///
/// `in_use` is advisory. It can change between this probe and a connection attempt, and the
/// server's own admission check remains the authority.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ApiConfigCapacity {
    // 0 is unlimited.
    pub limit: u32,
    pub in_use: u32,
}

impl Default for ApiConfigCapacity {
    fn default() -> Self {
        Self {
            limit: 0,
            in_use: 0,
        }
    }
}
