use std::fs;
use std::path::Path;

use anyhow::{Context, anyhow};
use serde::Deserialize;

use super::ServerSection;

/// The values of BVC's config.hcl that the launcher needs. Every other key is ignored.
#[derive(Debug, Default, Deserialize)]
pub struct BvcConfig {
    #[serde(default)]
    pub server: ServerSection,
}

impl BvcConfig {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content =
            fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        Self::from_hcl_str(&content)
    }

    /// Evaluates `${env.X}` the same way BVC does: an unset variable is an error.
    pub fn from_hcl_str(content: &str) -> anyhow::Result<Self> {
        let mut context = hcl::eval::Context::new();
        let env: hcl::Map<String, hcl::Value> = std::env::vars()
            .map(|(key, value)| (key, hcl::Value::String(value)))
            .collect();
        context.declare_var("env", hcl::Value::Object(env));
        hcl::eval::from_str(content, &context).map_err(|e| anyhow!("parsing config.hcl: {e}"))
    }

    /// The HTTPS address the BDS add-on posts to.
    pub fn server_url(&self) -> Option<String> {
        let tls = &self.server.tls;
        let host = tls
            .acme
            .as_ref()
            .and_then(|acme| acme.domains.as_ref())
            .and_then(|domains| domains.first())
            .or_else(|| tls.names.first())?;

        match self.server.port {
            443 => Some(format!("https://{host}")),
            port => Some(format!("https://{host}:{port}")),
        }
    }
}
