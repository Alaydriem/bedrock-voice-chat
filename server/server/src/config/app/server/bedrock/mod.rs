mod dns;
mod server_entry;

pub use dns::BedrockDnsConfig;
pub use server_entry::BedrockServerEntry;

use common::response::ApiConfigBedrock;
use serde::{Deserialize, Serialize};

fn default_enabled() -> bool {
    true
}

fn default_transfer_port() -> u16 {
    19132
}

fn default_transfer_target_port() -> u16 {
    19137
}

fn default_transfer_cache_ttl_secs() -> u64 {
    900
}

fn default_proxy_event_freshness_threshold_secs() -> u32 {
    30
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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
    pub dns: BedrockDnsConfig,
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
            dns: BedrockDnsConfig::default(),
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
            dns_enabled: self.dns.enabled,
            transfer_port: self.enabled.then_some(self.transfer_port),
            dns_override_host: (self.enabled && self.dns.enabled)
                .then(|| self.dns.override_host.clone()),
            servers: if self.enabled {
                self.servers.iter().map(BedrockServerEntry::to_api).collect()
            } else {
                Vec::new()
            },
        }
    }
}
