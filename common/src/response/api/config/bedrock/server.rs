use serde::{Deserialize, Serialize};
use ts_rs::TS;

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
}
