use serde::{Deserialize, Serialize};

use super::error::ConfigError;
use super::play_ca::PlayCaConfig;
use super::play_dns::PlayDnsConfig;

// The play plane: names under the play zone, the DNS server that answers for them,
// and the certificate authority that signs for them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayConfig {
    #[serde(default = "PlayConfig::default_zone")]
    pub zone: String,
    #[serde(default = "PlayConfig::default_claims_per_address_per_day")]
    pub claims_per_address_per_day: u32,
    #[serde(default = "PlayConfig::default_claims_per_hour")]
    pub claims_per_hour: u32,
    #[serde(default = "PlayConfig::default_release_after_days")]
    pub release_after_days: u32,
    // Certificates one node may be issued in 24 hours. Renewal is every few days, so
    // the default is generous.
    #[serde(default = "PlayConfig::default_issuances_per_node_per_day")]
    pub issuances_per_node_per_day: u32,
    // Seconds between play check-ins that start a probe. A check-in inside the window is
    // answered with the same nonce and probes nothing.
    #[serde(default = "PlayConfig::default_min_check_in_interval_secs")]
    pub min_check_in_interval_secs: u64,
    // Lets a check-in name a loopback, private or documentation address. For tests and
    // local runs only: on a public registry it turns the probe into a scanner of the
    // registry's own network.
    #[serde(default)]
    pub allow_private_probe_targets: bool,
    pub dns: PlayDnsConfig,
    pub ca: PlayCaConfig,
}

impl PlayConfig {
    // The only name the CA's own certificate is issued for.
    pub const CA_HOST: &'static str = "ca.play.bedrockvc.stream";

    // The only zone the client roots, the CA's name constraints and `CA_HOST` accept.
    pub const ZONE: &'static str = "play.bedrockvc.stream";

    fn default_zone() -> String {
        Self::ZONE.to_string()
    }

    fn default_claims_per_address_per_day() -> u32 {
        3
    }

    fn default_claims_per_hour() -> u32 {
        200
    }

    fn default_release_after_days() -> u32 {
        30
    }

    fn default_issuances_per_node_per_day() -> u32 {
        5
    }

    fn default_min_check_in_interval_secs() -> u64 {
        60
    }

    // A secondary pulls and is notified with the TSIG key, so secondaries without
    // one could neither transfer nor be told of a change.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if !self.dns.secondaries.is_empty() && self.dns.tsig.is_none() {
            return Err(ConfigError::Parse(
                "play.dns.secondaries requires play.dns.tsig".to_string(),
            ));
        }

        if !self.zone.trim_end_matches('.').eq_ignore_ascii_case(Self::ZONE) {
            return Err(ConfigError::Parse(format!(
                "play.zone must be {}, the only zone the play roots permit",
                Self::ZONE
            )));
        }

        if self.dns.bind.is_empty() {
            return Err(ConfigError::Parse(
                "play.dns.bind must name at least one address".to_string(),
            ));
        }

        if self.dns.nameservers.is_empty() {
            return Err(ConfigError::Parse(
                "play.dns.nameservers must name at least one server".to_string(),
            ));
        }

        if let Some(tsig) = &self.dns.tsig {
            tsig.signer()?;
        }

        let names = std::iter::once(("play.zone", &self.zone))
            .chain(std::iter::once(("play.dns.primary", &self.dns.primary)))
            .chain(std::iter::once(("play.dns.hostmaster", &self.dns.hostmaster)))
            .chain(self.dns.nameservers.iter().map(|n| ("play.dns.nameservers", n)));
        for (field, value) in names {
            hickory_proto::rr::Name::from_ascii(value)
                .map_err(|e| ConfigError::Parse(format!("{field} is not a DNS name: {e}")))?;
        }

        self.validate_ca_url()
    }

    // The CA is reached by a fixed name that is resolved to `connect`, and reqwest uses
    // the URL's port rather than the socket address's. A URL naming another host would be
    // resolved by public DNS; another port would silently ignore `connect`.
    fn validate_ca_url(&self) -> Result<(), ConfigError> {
        let url = url::Url::parse(&self.ca.url)
            .map_err(|e| ConfigError::Parse(format!("play.ca.url is not a URL: {e}")))?;

        if url.host_str() != Some(Self::CA_HOST) {
            return Err(ConfigError::Parse(format!(
                "play.ca.url must name the host {}",
                Self::CA_HOST
            )));
        }

        if url.port_or_known_default() != Some(self.ca.connect.port()) {
            return Err(ConfigError::Parse(
                "play.ca.url must use the same port as play.ca.connect".to_string(),
            ));
        }

        Ok(())
    }
}
