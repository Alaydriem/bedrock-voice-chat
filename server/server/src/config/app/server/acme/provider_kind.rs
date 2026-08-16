use std::fmt;
use std::str::FromStr;

use anyhow::anyhow;
use serde::{Deserialize, Serialize};

/// Which DNS provider fulfills the DNS-01 challenge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(try_from = "String", into = "String")]
#[schemars(with = "String")]
pub enum AcmeProviderKind {
    Cloudflare,
    AcmeDns,
}

impl AcmeProviderKind {
    pub const SUPPORTED: &str = "cloudflare, acme-dns";

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Cloudflare => "cloudflare",
            Self::AcmeDns => "acme-dns",
        }
    }
}

impl FromStr for AcmeProviderKind {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "cloudflare" => Ok(Self::Cloudflare),
            "acme-dns" => Ok(Self::AcmeDns),
            other => Err(anyhow!(
                "unknown acme provider {other:?}; supported: {}",
                Self::SUPPORTED
            )),
        }
    }
}

impl TryFrom<String> for AcmeProviderKind {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<AcmeProviderKind> for String {
    fn from(kind: AcmeProviderKind) -> Self {
        kind.as_str().to_string()
    }
}

impl fmt::Display for AcmeProviderKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
