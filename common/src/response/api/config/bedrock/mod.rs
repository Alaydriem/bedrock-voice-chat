pub mod server;

pub use server::ApiConfigBedrockServer;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ApiConfigBedrock {
    // Whether the server supports Bedrock features. Always true on this server,
    // and kept on the wire because a client defaults a missing value to false
    // and would hide its Proxy and Realms Connect pages.
    #[serde(default)]
    pub enabled: bool,
    // Operator-curated Bedrock servers for the client's Proxy Connect list.
    #[serde(default)]
    pub servers: Vec<ApiConfigBedrockServer>,
}
