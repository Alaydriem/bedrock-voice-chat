use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::BedrockConnectMode;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct BedrockConnectConfig {
    pub mode: BedrockConnectMode,
    pub proxy_host: Option<String>,
    pub proxy_port: Option<u16>,
    pub realm_id: Option<u64>,
    pub network_interface: Option<String>,
}
