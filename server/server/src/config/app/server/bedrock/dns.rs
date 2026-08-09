use serde::{Deserialize, Serialize};

fn default_dns_enabled() -> bool {
    false
}

fn default_dns_port() -> u16 {
    53
}

fn default_dns_upstream() -> Vec<String> {
    vec!["1.1.1.1".to_string(), "1.0.0.1".to_string()]
}

fn default_dns_override_host() -> String {
    "geo.hivebedrock.network".to_string()
}

fn default_rate_limit_per_sec() -> u32 {
    100
}

#[derive(Serialize, Deserialize, Debug, Clone, schemars::JsonSchema)]
pub struct BedrockDnsConfig {
    #[serde(default = "default_dns_enabled")]
    pub enabled: bool,
    #[serde(default = "default_dns_port")]
    pub port: u16,
    #[serde(default = "default_dns_upstream")]
    pub upstream: Vec<String>,
    #[serde(default = "default_dns_override_host")]
    pub override_host: String,
    #[serde(default = "default_rate_limit_per_sec")]
    pub rate_limit_per_sec: u32,
}

impl Default for BedrockDnsConfig {
    fn default() -> Self {
        Self {
            enabled: default_dns_enabled(),
            port: default_dns_port(),
            upstream: default_dns_upstream(),
            override_host: default_dns_override_host(),
            rate_limit_per_sec: default_rate_limit_per_sec(),
        }
    }
}
