mod dns;

pub use dns::BedrockDnsConfig;

use serde::{Deserialize, Serialize};

fn default_enabled() -> bool {
    false
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
    #[serde(default)]
    pub dns: BedrockDnsConfig,
}

impl Default for BedrockConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            transfer_port: default_transfer_port(),
            transfer_target_port: default_transfer_target_port(),
            transfer_cache_ttl_secs: default_transfer_cache_ttl_secs(),
            dns: BedrockDnsConfig::default(),
        }
    }
}
