mod acme;
mod cloudflare;
mod discord;
mod error;
mod http;
mod logger;

pub use acme::AcmeConfig;
pub use cloudflare::CloudflareConfig;
pub use discord::DiscordConfig;
pub use error::ConfigError;
pub use http::HttpConfig;
pub use logger::LoggerConfig;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayConfig {
    pub database_url: String,
    // The apex the relay assigns names under. Names sit directly beneath it.
    pub zone: String,
    pub discord: DiscordConfig,
    pub cloudflare: CloudflareConfig,

    // The operator-facing HTTP surface: Discord's redirect target and the API the
    // enrollment page calls.
    #[serde(default)]
    pub http: HttpConfig,

    // Console and a rotating JSON file. Console is unconditional.
    #[serde(default)]
    pub logger: LoggerConfig,

    // The UDP port the enrollment endpoint binds. Pinned rather than left to the
    // operating system, because every server's stored relay address names it.
    #[serde(default = "RelayConfig::default_enroll_port")]
    pub enroll_port: u16,
    // First issuances the certificate authority will accept per week for this zone.
    // Renewals are exempt and are not counted against it.
    #[serde(default = "RelayConfig::default_weekly_certificate_ceiling")]
    pub weekly_certificate_ceiling: u32,
}

impl RelayConfig {
    fn default_enroll_port() -> u16 {
        28286
    }

    fn default_weekly_certificate_ceiling() -> u32 {
        50
    }

    /// Parses an HCL document, evaluating `${env.VAR}` expressions against the
    /// supplied variable map.
    ///
    /// A referenced-but-unset variable is a hard error rather than a silent empty
    /// string. Every secret this file carries — the bot token, the Cloudflare token —
    /// arrives that way, and one that resolved to nothing would start a relay that
    /// authenticates against neither service and reports the reason only at the first
    /// request.
    pub fn from_hcl_with_env(
        source: &str,
        env: &HashMap<String, String>,
    ) -> Result<Self, ConfigError> {
        let mut ctx = hcl::eval::Context::new();
        let env_object: hcl::Map<String, hcl::Value> = env
            .iter()
            .map(|(k, v)| (k.clone(), hcl::Value::String(v.clone())))
            .collect();
        ctx.declare_var("env", hcl::Value::Object(env_object));

        let value: serde_json::Value =
            hcl::eval::from_str(source, &ctx).map_err(|e| ConfigError::Parse(e.to_string()))?;
        serde_json::from_value(value).map_err(|e| ConfigError::Parse(e.to_string()))
    }

    /// Parses an HCL document, exposing the full process environment as the `env`
    /// object so any `${env.VAR}` reference resolves.
    pub fn from_hcl(source: &str) -> Result<Self, ConfigError> {
        Self::from_hcl_with_env(source, &std::env::vars().collect())
    }

    pub fn from_path(path: &str) -> Result<Self, ConfigError> {
        let source = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.to_string(),
            source,
        })?;
        Self::from_hcl(&source)
    }
}
