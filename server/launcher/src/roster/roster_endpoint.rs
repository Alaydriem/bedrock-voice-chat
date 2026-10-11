use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, anyhow};
use reqwest::Url;
use reqwest::blocking::Client;

use crate::setup::VariablesFile;

/// Where the roster is sent: BVC's address and the game access token from `variables.json`.
///
/// A hostname is dialled on loopback with TLS still verified against that name, so the
/// launcher never depends on the host reaching its own public address.
pub struct RosterEndpoint {
    base: Url,
    token: String,
}

impl RosterEndpoint {
    pub fn from_variables(path: &Path) -> anyhow::Result<Self> {
        let variables = VariablesFile::load(path)?;
        let server = variables
            .get(VariablesFile::SERVER_KEY)
            .ok_or_else(|| anyhow!("{} is not set", VariablesFile::SERVER_KEY))?;
        let token = variables
            .get(VariablesFile::TOKEN_KEY)
            .ok_or_else(|| anyhow!("{} is not set", VariablesFile::TOKEN_KEY))?;
        Ok(Self {
            base: Url::parse(server).with_context(|| format!("parsing {server}"))?,
            token: token.to_string(),
        })
    }

    pub fn url(&self, path: &str) -> anyhow::Result<Url> {
        self.base
            .join(path)
            .with_context(|| format!("joining {path}"))
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    pub fn client(&self, timeout: Duration) -> anyhow::Result<Client> {
        let mut builder = Client::builder().timeout(timeout);
        if let (Some(host), Some(port)) = (self.base.host_str(), self.base.port_or_known_default())
            && host.parse::<IpAddr>().is_err()
        {
            builder =
                builder.resolve(host, SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port));
        }
        Ok(builder.build()?)
    }
}
