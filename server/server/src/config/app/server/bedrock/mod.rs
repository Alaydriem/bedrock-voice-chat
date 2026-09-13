mod server_entry;

pub use server_entry::BedrockServerEntry;

use common::response::ApiConfigBedrock;
use serde::{Deserialize, Serialize};

fn default_proxy_event_freshness_threshold_secs() -> u32 {
    30
}

#[derive(Serialize, Deserialize, Debug, Clone, schemars::JsonSchema)]
pub struct BedrockConfig {
    #[serde(default = "default_proxy_event_freshness_threshold_secs")]
    pub proxy_event_freshness_threshold_secs: u32,
    #[serde(default)]
    pub servers: Vec<BedrockServerEntry>,
}

impl Default for BedrockConfig {
    fn default() -> Self {
        Self {
            proxy_event_freshness_threshold_secs: default_proxy_event_freshness_threshold_secs(),
            servers: Vec::new(),
        }
    }
}

impl BedrockConfig {
    // Wire-facing view of this config for `/api/config`. `enabled` is a constant:
    // Bedrock support is not something an operator can turn off, and the field
    // survives only because a client that reads no value defaults it to false.
    pub fn to_api(&self) -> ApiConfigBedrock {
        ApiConfigBedrock {
            enabled: true,
            servers: self.servers.iter().map(BedrockServerEntry::to_api).collect(),
        }
    }
}
