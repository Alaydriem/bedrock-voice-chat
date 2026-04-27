use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct BedrockStatus {
    pub proxy_running: bool,
    pub realms_running: bool,
    pub xbox_authenticated: bool,
    pub proxy_target_host: Option<String>,
    pub proxy_target_port: Option<u16>,
    pub proxy_listen_port: Option<u16>,
    pub active_realm_id: Option<u64>,
    pub active_realm_name: Option<String>,
}
