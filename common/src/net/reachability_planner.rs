use std::net::IpAddr;

use anyhow::anyhow;
use url::Url;

use crate::structs::network::QuicPortSelection;
use crate::structs::reachability::ReachabilityRequest;

pub struct ReachabilityPlanner;

impl ReachabilityPlanner {
    const CONFIG_PATH: &'static str = "/api/config";

    // A bare host is what the login field holds before it is sanitized, so both
    // forms have to parse. Url::parse rather than a string split: an IPv6 literal
    // is bracketed and colon-separated, and splitting on ':' returns "[".
    pub fn host_of(server_url: &str) -> Result<String, anyhow::Error> {
        let trimmed = server_url.trim();
        let absolute = if trimmed.contains("://") {
            trimmed.to_string()
        } else {
            format!("https://{}", trimmed)
        };

        let parsed = Url::parse(&absolute)
            .map_err(|e| anyhow!("{} is not a usable address: {}", server_url, e))?;

        match parsed.host() {
            Some(url::Host::Ipv6(addr)) => Ok(format!("[{}]", addr)),
            Some(_) => Ok(parsed
                .host_str()
                .expect("a parsed host always has a string form")
                .to_string()),
            None => Err(anyhow!("{} carries no host", server_url)),
        }
    }

    pub fn ports(
        advertised_ports: &[u32],
        advertised_scalar: u32,
        cached_connect_string: Option<&str>,
    ) -> Vec<u16> {
        QuicPortSelection::resolve(advertised_ports, advertised_scalar, cached_connect_string)
    }

    pub fn request(
        host: String,
        addrs: Vec<IpAddr>,
        ports: Vec<u16>,
        server_url: &str,
    ) -> ReachabilityRequest {
        let trimmed = server_url.trim_end_matches('/');
        let https_port = Url::parse(server_url)
            .ok()
            .and_then(|u| u.port())
            .unwrap_or(ReachabilityRequest::DEFAULT_HTTPS_PORT);

        ReachabilityRequest::new(
            host,
            addrs,
            ports,
            format!("{}{}", trimmed, Self::CONFIG_PATH),
            https_port,
        )
    }

    // One DNS lookup serves every candidate: the host is identical and only the
    // port varies. Both families are kept — collapsing to a single IPv4 address
    // is what left an IPv6-only host with nothing to dial.
    pub async fn plan(
        server_url: &str,
        advertised_ports: &[u32],
        advertised_scalar: u32,
        cached_connect_string: Option<&str>,
    ) -> Result<ReachabilityRequest, anyhow::Error> {
        let host = Self::host_of(server_url)?;
        let ports = Self::ports(advertised_ports, advertised_scalar, cached_connect_string);

        let resolved = tokio::net::lookup_host(format!("{}:{}", host, ports[0]))
            .await
            .map_err(|e| anyhow!("DNS_FAIL: {}", e))?;

        let mut addrs: Vec<IpAddr> = Vec::new();
        for addr in resolved {
            if !addrs.contains(&addr.ip()) {
                addrs.push(addr.ip());
            }
        }

        if addrs.is_empty() {
            return Err(anyhow!("DNS_FAIL: System DNS returned no results"));
        }

        Ok(Self::request(host, addrs, ports, server_url))
    }
}
