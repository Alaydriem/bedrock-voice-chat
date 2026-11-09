use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

use super::play_secondary::PlaySecondaryConfig;
use super::play_tsig::PlayTsigConfig;

// The authoritative DNS server for the play zone.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayDnsConfig {
    // Required. There is no wildcard default: a wildcard bind collides with a local
    // stub resolver, and an operator must name the address on purpose.
    pub bind: Vec<SocketAddr>,
    // The public NS set. Equal to the delegation at the parent zone.
    pub nameservers: Vec<String>,
    // The SOA `mname`: the registry's own NS name, listed or hidden.
    pub primary: String,
    pub hostmaster: String,
    #[serde(default = "PlayDnsConfig::default_ttl")]
    pub ttl: u32,
    #[serde(default)]
    pub secondaries: Vec<PlaySecondaryConfig>,
    #[serde(default)]
    pub tsig: Option<PlayTsigConfig>,
    #[serde(default = "PlayDnsConfig::default_notify_delay_secs")]
    pub notify_delay_secs: u64,
}

impl PlayDnsConfig {
    fn default_ttl() -> u32 {
        60
    }

    fn default_notify_delay_secs() -> u64 {
        5
    }
}
