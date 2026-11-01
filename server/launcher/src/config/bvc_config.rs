use std::fs;
use std::path::Path;

use anyhow::{Context, anyhow};
use serde::Deserialize;

use super::ServerSection;

/// The values of BVC's config (`config/bvc.hcl`) that the launcher needs. Every other key is ignored.
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
        hcl::eval::from_str(content, &context).map_err(|e| anyhow!("parsing bvc.hcl: {e}"))
    }

    /// Whether `server.tls.play` is present.
    pub fn is_play_mode(&self) -> bool {
        self.server.tls.play.is_some()
    }

    /// Whether a value is set to something other than blank. The server reads a blank value
    /// as unset.
    pub fn is_set(value: Option<&str>) -> bool {
        value.is_some_and(|value| !value.trim().is_empty())
    }

    /// Whether the config names no way to obtain a certificate: no `tls.certificate`, no
    /// `tls.acme`, no `enrollment.token` and no `tls.play`. `environment_token` is the
    /// `BVC_ENROLLMENT_TOKEN` value, which the server also accepts. Blank values are unset.
    pub fn names_no_certificate_source(&self, environment_token: Option<&str>) -> bool {
        let tls = &self.server.tls;
        !Self::is_set(tls.certificate.as_deref())
            && tls.acme.is_none()
            && tls.play.is_none()
            && !Self::is_set(self.server.enrollment.token.as_deref())
            && !Self::is_set(environment_token)
    }

    /// The advice to print for a config that names no certificate source, if it needs any.
    pub fn certificate_source_warning(&self, environment_token: Option<&str>) -> Option<&'static str> {
        self.names_no_certificate_source(environment_token).then_some(
            "config/bvc.hcl names no certificate source; to use play mode add `mod_loopback = true` and `tls { play {} }` under `server`: server { mod_loopback = true  tls { play {} } }",
        )
    }

    /// The address the BDS add-on posts to. In play mode with `server.mod_loopback` on, the
    /// add-on reaches the server over loopback in plain HTTP; otherwise it is the HTTPS name.
    pub fn server_url(&self) -> Option<String> {
        if self.is_play_mode() && self.server.mod_loopback {
            return Some(format!("http://127.0.0.1:{}", self.server.port));
        }

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
