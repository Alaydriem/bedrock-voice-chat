mod acme_dns;
mod cloudflare;

pub use acme_dns::AcmeDnsProvider;
pub use cloudflare::CloudflareProvider;

use anyhow::{Result, anyhow};

use crate::config::{Acme, AcmeProviderKind};

/// The DNS backends that can fulfill a DNS-01 challenge. Enum delegation —
/// never trait objects.
pub enum DnsProvider {
    Cloudflare(CloudflareProvider),
    AcmeDns(AcmeDnsProvider),
}

impl DnsProvider {
    /// Builds the configured provider. `Acme::validate` has already enforced
    /// field presence; the errors here restate that contract.
    pub fn from_config(acme: &Acme) -> Result<Self> {
        match acme.provider_kind()? {
            AcmeProviderKind::Cloudflare => {
                let token = acme
                    .api_token
                    .as_deref()
                    .ok_or_else(|| anyhow!("acme.api_token is required for cloudflare"))?;
                Ok(Self::Cloudflare(CloudflareProvider::new(token)))
            }
            AcmeProviderKind::AcmeDns => {
                let server_url = acme
                    .server_url
                    .as_deref()
                    .ok_or_else(|| anyhow!("acme.server_url is required for acme-dns"))?;
                let username = acme
                    .username
                    .as_deref()
                    .ok_or_else(|| anyhow!("acme.username is required for acme-dns"))?;
                let password = acme
                    .password
                    .as_deref()
                    .ok_or_else(|| anyhow!("acme.password is required for acme-dns"))?;
                let subdomain = acme
                    .subdomain
                    .as_deref()
                    .ok_or_else(|| anyhow!("acme.subdomain is required for acme-dns"))?;
                Ok(Self::AcmeDns(AcmeDnsProvider::new(
                    server_url, username, password, subdomain,
                )))
            }
        }
    }

    pub async fn publish_txt(&self, domain: &str, value: &str) -> Result<()> {
        match self {
            Self::Cloudflare(p) => p.publish_txt(domain, value).await,
            Self::AcmeDns(p) => p.publish_txt(domain, value).await,
        }
    }

    pub async fn cleanup_txt(&self, domain: &str) -> Result<()> {
        match self {
            Self::Cloudflare(p) => p.cleanup_txt(domain).await,
            Self::AcmeDns(p) => p.cleanup_txt(domain).await,
        }
    }
}
