use std::net::IpAddr;

use anyhow::{Result, anyhow};
use url::{Host, Url};

use crate::consts::PLAY_ZONE_SUFFIX;

/// What a user typed to name a server: a host, and optionally a port.
///
/// `identity()` is the stored string byte for byte, and stays the key for everything
/// that is stored or pooled per server.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerAddress {
    identity: String,
    host: String,
    ip_literal: bool,
    explicit_port: Option<u16>,
}

impl ServerAddress {
    pub fn parse(stored: &str) -> Result<Self> {
        let trimmed = stored.trim();
        let with_scheme = if trimmed.contains("://") {
            trimmed.to_string()
        } else {
            format!("https://{trimmed}")
        };

        let url = Url::parse(&with_scheme).map_err(|e| anyhow!("invalid server address: {e}"))?;
        if !matches!(url.scheme(), "http" | "https") {
            return Err(anyhow!("invalid server address: unsupported scheme"));
        }

        let (host, ip_literal) = match url.host() {
            Some(Host::Domain(domain)) if !domain.is_empty() => (domain.to_ascii_lowercase(), false),
            Some(Host::Ipv4(ip)) => (ip.to_string(), true),
            Some(Host::Ipv6(ip)) => (ip.to_string(), true),
            _ => return Err(anyhow!("invalid server address: no host")),
        };

        // `Url::port` hides a port equal to the scheme default, so a typed `:443` is read
        // from the authority text instead.
        let explicit_port = if Self::authority_has_port(&with_scheme) {
            url.port_or_known_default()
        } else {
            None
        };
        if explicit_port == Some(0) {
            return Err(anyhow!("invalid server address: port 0"));
        }

        Ok(Self {
            identity: stored.to_string(),
            host,
            ip_literal,
            explicit_port,
        })
    }

    fn authority_has_port(with_scheme: &str) -> bool {
        let after_scheme = with_scheme.split_once("://").map_or(with_scheme, |(_, rest)| rest);
        let authority = after_scheme
            .split(['/', '?', '#'])
            .next()
            .unwrap_or_default();
        let host_part = authority.rsplit_once('@').map_or(authority, |(_, host)| host);

        match host_part.strip_prefix('[') {
            Some(rest) => rest
                .split_once(']')
                .is_some_and(|(_, after)| after.starts_with(':')),
            None => host_part.contains(':'),
        }
    }

    pub fn identity(&self) -> &str {
        &self.identity
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn url_host(&self) -> String {
        match self.host.parse::<IpAddr>() {
            Ok(IpAddr::V6(_)) => format!("[{}]", self.host),
            _ => self.host.clone(),
        }
    }

    pub fn explicit_port(&self) -> Option<u16> {
        self.explicit_port
    }

    pub fn is_ip_literal(&self) -> bool {
        self.ip_literal
    }

    pub fn is_play_name(&self) -> bool {
        !self.ip_literal
            && self.host.len() > PLAY_ZONE_SUFFIX.len()
            && self.host.ends_with(PLAY_ZONE_SUFFIX)
    }
}
