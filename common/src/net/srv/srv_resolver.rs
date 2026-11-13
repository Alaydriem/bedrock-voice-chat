use std::sync::Arc;
use std::time::Duration;

use super::{
    AuthoritativeQuery, DohLookup, SrvAnswer, SrvFallback, SrvLookup, SrvRecordChoice,
    SrvResolution, SrvResolverConfig,
};
#[cfg(not(any(target_os = "android", target_os = "ios")))]
use super::SystemLookup;
use crate::structs::network::{EndpointSource, ServerAddress, ServerEndpoint};

/// Finds the port a server listens on: the typed port, an SRV record, or 443.
pub struct SrvResolver {
    authoritative: AuthoritativeQuery,
    fallback: SrvFallback,
    doh: Option<DohLookup>,
}

impl SrvResolver {
    pub const AUTHORITATIVE_TIMEOUT: Duration = Duration::from_millis(1500);
    pub const FALLBACK_TIMEOUT: Duration = Duration::from_secs(2);
    pub const DEFAULT_PORT: u16 = 443;

    pub fn new(config: SrvResolverConfig) -> Self {
        let doh = match &config.fallback {
            SrvFallback::Https(url) => Some(DohLookup::new(url.clone(), Self::FALLBACK_TIMEOUT)),
            _ => None,
        };

        Self {
            authoritative: AuthoritativeQuery::new(
                config.nameservers,
                Self::AUTHORITATIVE_TIMEOUT,
            ),
            fallback: config.fallback,
            doh,
        }
    }

    pub fn new_shared(config: SrvResolverConfig) -> Arc<Self> {
        Arc::new(Self::new(config))
    }

    pub async fn resolve(&self, address: &ServerAddress) -> SrvResolution {
        if let Some(port) = address.explicit_port() {
            return Self::resolution(address, port, EndpointSource::Explicit, None);
        }
        if address.is_ip_literal() {
            return Self::default_resolution(address);
        }

        let host = address.host();
        let mut lookup = None;
        let mut failed = false;

        if address.is_play_name() {
            match self.authoritative.lookup(host).await {
                SrvLookup::Failed(reason) => {
                    Self::log_failed_step(host, "authoritative", &reason);
                    failed = true;
                }
                settled => lookup = Some(settled),
            }
        }

        if lookup.is_none()
            && let Some((step, found)) = self.fallback_lookup(host).await
        {
            match found {
                SrvLookup::Failed(reason) => {
                    Self::log_failed_step(host, step, &reason);
                    failed = true;
                }
                settled => lookup = Some(settled),
            }
        }

        match lookup {
            Some(SrvLookup::Found(answers)) => match SrvRecordChoice::pick(host, &answers) {
                Some(chosen) => Self::resolution(
                    address,
                    chosen.port,
                    EndpointSource::Srv,
                    Some(Duration::from_secs(u64::from(chosen.ttl))),
                ),
                None => {
                    Self::log_ignored_target(host, &answers);
                    Self::default_resolution(address)
                }
            },
            Some(_) => Self::default_resolution(address),
            None => {
                if failed {
                    curia::info!("SRV lookup failed; using port 443", { "host": host.to_string() });
                }
                Self::default_resolution(address)
            }
        }
    }

    async fn fallback_lookup(&self, host: &str) -> Option<(&'static str, SrvLookup)> {
        match &self.fallback {
            #[cfg(not(any(target_os = "android", target_os = "ios")))]
            SrvFallback::System => Some((
                "system",
                SystemLookup::new(Self::FALLBACK_TIMEOUT).lookup(host).await,
            )),
            SrvFallback::Https(_) => match &self.doh {
                Some(doh) => Some(("https", doh.lookup(host).await)),
                None => None,
            },
            SrvFallback::None => None,
        }
    }

    fn resolution(
        address: &ServerAddress,
        port: u16,
        source: EndpointSource,
        ttl: Option<Duration>,
    ) -> SrvResolution {
        SrvResolution {
            endpoint: ServerEndpoint::new(address.clone(), port, source),
            ttl,
        }
    }

    fn default_resolution(address: &ServerAddress) -> SrvResolution {
        Self::resolution(address, Self::DEFAULT_PORT, EndpointSource::Default, None)
    }

    fn log_failed_step(host: &str, step: &str, reason: &str) {
        curia::debug!("SRV lookup step failed", {
            "host": host.to_string(),
            "step": step.to_string(),
            "reason": reason.to_string()
        });
    }

    // A record that names another host, or the root, is dropped by the choice; only the other
    // host is worth telling the operator about.
    fn log_ignored_target(host: &str, answers: &[SrvAnswer]) {
        let wanted = host.trim_end_matches('.');
        let other = answers.iter().find(|answer| {
            let target = answer.target.trim_end_matches('.');
            !target.is_empty() && !target.eq_ignore_ascii_case(wanted)
        });
        if let Some(answer) = other {
            curia::info!("SRV record ignored: its target is a different host", {
                "host": host.to_string(),
                "target": answer.target.clone()
            });
        }
    }
}
