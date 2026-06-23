use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::ApiConfigResponse;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ApiConfigCheckResponse {
    pub config: ApiConfigResponse,
    pub client_version: String,
    pub compatible: bool,
    pub client_too_old: bool,
}

impl ApiConfigCheckResponse {
    pub fn from_config(config: ApiConfigResponse, client_version: &str) -> Self {
        let server_parts: Vec<u32> = config
            .protocol_version
            .split('.')
            .filter_map(|s| s.parse().ok())
            .collect();
        let client_parts: Vec<u32> = client_version
            .split('.')
            .filter_map(|s| s.parse().ok())
            .collect();

        let server_major = server_parts.first().copied().unwrap_or(0);
        let server_minor = server_parts.get(1).copied().unwrap_or(0);
        let client_major = client_parts.first().copied().unwrap_or(0);
        let client_minor = client_parts.get(1).copied().unwrap_or(0);

        let compatible = server_major == client_major && server_minor == client_minor;
        let client_too_old = (client_major, client_minor) < (server_major, server_minor);

        Self {
            config,
            client_version: client_version.to_string(),
            compatible,
            client_too_old,
        }
    }
}
