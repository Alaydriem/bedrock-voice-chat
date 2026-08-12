use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::structs::bedrock::AddonMode;

// One operator-curated Bedrock server advertised via `/api/config`. The client
// shows these as read-only, pre-populated entries in its Proxy Connect list.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ApiConfigBedrockServer {
    pub name: String,
    pub host: String,
    pub port: u16,
    // Raw Bedrock protocol version the client proxy should advertise for this
    // server. None means Auto — mirror the real backend's version.
    #[serde(default)]
    pub protocol_version: Option<u32>,
    // Who owns event delivery for this world. Required: an advertised server
    // whose mode nobody declared is an operator mistake, not a value to guess.
    pub addon_mode: AddonMode,
}
