pub mod server;

pub use server::ApiConfigBedrockServer;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ApiConfigBedrock {
    // Whether the server runs the Bedrock transfer relay (feature compiled in
    // and enabled). Clients gate their Proxy/Realms Connect pages on this: when
    // false they render an unsupported-server notice instead of the feature UI.
    #[serde(default)]
    pub enabled: bool,
    // Transfer relay entry port, present only when `enabled`. Clients advertise
    // this as a connection option distinct from the local proxy listen port.
    #[serde(default)]
    pub transfer_port: Option<u16>,
    // Operator-curated Bedrock servers for the client's Proxy Connect list.
    // Populated only when `enabled`.
    #[serde(default)]
    pub servers: Vec<ApiConfigBedrockServer>,
}
