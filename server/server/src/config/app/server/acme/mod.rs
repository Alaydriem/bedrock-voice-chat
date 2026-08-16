mod provider_kind;

pub use provider_kind::AcmeProviderKind;

use anyhow::anyhow;
use serde::{Deserialize, Serialize};

fn default_directory() -> String {
    "https://acme-v02.api.letsencrypt.org/directory".to_string()
}

/// ACME DNS-01 configuration. Mutually exclusive with manual
/// `tls.certificate`/`tls.key` — the server refuses to start with both set.
#[derive(Serialize, Deserialize, Debug, Clone, schemars::JsonSchema)]
pub struct Acme {
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub provider: Option<AcmeProviderKind>,
    // Cloudflare: zone-scoped API token with DNS edit permission.
    #[serde(default)]
    pub api_token: Option<String>,
    // acme-dns: registration credentials and the delegated subdomain.
    #[serde(default)]
    pub server_url: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub subdomain: Option<String>,
    #[serde(default = "default_directory")]
    pub directory: String,
    // Defaults to the DNS entries of tls.names when absent.
    #[serde(default)]
    pub domains: Option<Vec<String>>,
}

impl Default for Acme {
    fn default() -> Self {
        Self {
            email: String::new(),
            provider: None,
            api_token: None,
            server_url: None,
            username: None,
            password: None,
            subdomain: None,
            directory: default_directory(),
            domains: None,
        }
    }
}

impl Acme {
    pub fn provider_kind(&self) -> Result<AcmeProviderKind, anyhow::Error> {
        self.provider.ok_or_else(|| {
            anyhow!(
                "acme.provider is required; supported: {}",
                AcmeProviderKind::SUPPORTED
            )
        })
    }

    /// The domains to put on the certificate: the explicit `domains` list, or
    /// the DNS entries of `tls.names` (IP entries cannot appear on an ACME
    /// certificate and are skipped).
    pub fn effective_domains(&self, tls_names: &[String]) -> Result<Vec<String>, anyhow::Error> {
        let domains: Vec<String> = match &self.domains {
            Some(list) => list.clone(),
            None => tls_names
                .iter()
                .filter(|name| name.parse::<std::net::IpAddr>().is_err())
                .cloned()
                .collect(),
        };
        if domains.is_empty() {
            return Err(anyhow!(
                "acme is enabled but no DNS domain is available (tls.names has no DNS entries and acme.domains is unset)"
            ));
        }
        Ok(domains)
    }

    /// Full startup validation: email, provider, provider-specific fields,
    /// and a non-empty effective domain list.
    pub fn validate(&self, tls_names: &[String]) -> Result<(), anyhow::Error> {
        if self.email.trim().is_empty() {
            return Err(anyhow!("acme.email is required"));
        }
        let kind = self.provider_kind()?;
        let required: Vec<(&str, &Option<String>)> = match kind {
            AcmeProviderKind::Cloudflare => vec![("api_token", &self.api_token)],
            AcmeProviderKind::AcmeDns => vec![
                ("server_url", &self.server_url),
                ("username", &self.username),
                ("password", &self.password),
                ("subdomain", &self.subdomain),
            ],
        };
        let missing: Vec<&str> = required
            .into_iter()
            .filter(|(_, value)| value.as_deref().unwrap_or("").trim().is_empty())
            .map(|(name, _)| name)
            .collect();
        if !missing.is_empty() {
            return Err(anyhow!(
                "acme provider {kind} is missing required fields: {}",
                missing.join(", ")
            ));
        }
        self.effective_domains(tls_names)?;
        Ok(())
    }
}
