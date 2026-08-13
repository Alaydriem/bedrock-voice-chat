mod server_entry;

pub use server_entry::BedrockServerEntry;

use common::response::ApiConfigBedrock;
use serde::{Deserialize, Serialize};

fn default_enabled() -> bool {
    true
}

fn default_transfer_port() -> u16 {
    28283
}

// The relay sends players to the client proxy, so this is not an independent
// setting — two numbers that must agree is the drift being removed.
fn default_transfer_target_port() -> u16 {
    common::consts::bedrock::BEDROCK_LISTEN_PORT
}

fn default_transfer_cache_ttl_secs() -> u64 {
    900
}

fn default_proxy_event_freshness_threshold_secs() -> u32 {
    30
}

#[derive(Serialize, Deserialize, Debug, Clone, schemars::JsonSchema)]
pub struct BedrockConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_transfer_port")]
    pub transfer_port: u16,
    #[serde(default = "default_transfer_target_port")]
    pub transfer_target_port: u16,
    #[serde(default = "default_transfer_cache_ttl_secs")]
    pub transfer_cache_ttl_secs: u64,
    #[serde(default = "default_proxy_event_freshness_threshold_secs")]
    pub proxy_event_freshness_threshold_secs: u32,
    #[serde(default)]
    pub servers: Vec<BedrockServerEntry>,
}

impl Default for BedrockConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            transfer_port: default_transfer_port(),
            transfer_target_port: default_transfer_target_port(),
            transfer_cache_ttl_secs: default_transfer_cache_ttl_secs(),
            proxy_event_freshness_threshold_secs: default_proxy_event_freshness_threshold_secs(),
            servers: Vec::new(),
        }
    }
}

impl BedrockConfig {
    // Wire-facing view of this config for `/api/config`. The transfer port and
    // the curated server list are withheld when the relay is disabled: a
    // disabled relay has nothing a client can connect to.
    pub fn to_api(&self) -> ApiConfigBedrock {
        ApiConfigBedrock {
            enabled: self.enabled,
            transfer_port: self.enabled.then_some(self.transfer_port),
            servers: if self.enabled {
                self.servers.iter().map(BedrockServerEntry::to_api).collect()
            } else {
                Vec::new()
            },
        }
    }
}
