use std::net::IpAddr;

use anyhow::anyhow;

use crate::structs::network::{QuicPortSelection, ServerEndpoint};
use crate::structs::reachability::ReachabilityRequest;

pub struct ReachabilityPlanner;

impl ReachabilityPlanner {
    const CONFIG_PATH: &'static str = "/api/config";

    pub fn ports(
        advertised_ports: &[u32],
        advertised_scalar: u32,
        cached_connect_string: Option<&str>,
    ) -> Vec<u16> {
        QuicPortSelection::resolve(advertised_ports, advertised_scalar, cached_connect_string)
    }

    pub fn request(
        addrs: Vec<IpAddr>,
        ports: Vec<u16>,
        endpoint: &ServerEndpoint,
        voice_websocket: bool,
    ) -> ReachabilityRequest {
        ReachabilityRequest::new(
            endpoint.address().url_host(),
            addrs,
            ports,
            format!("{}{}", endpoint.base_url(), Self::CONFIG_PATH),
            endpoint.port(),
            voice_websocket,
        )
    }

    // One DNS lookup serves every candidate: the host is identical and only the
    // port varies. Both families are kept — collapsing to a single IPv4 address
    // is what left an IPv6-only host with nothing to dial.
    pub async fn plan(
        endpoint: &ServerEndpoint,
        advertised_ports: &[u32],
        advertised_scalar: u32,
        cached_connect_string: Option<&str>,
        voice_websocket: bool,
    ) -> Result<ReachabilityRequest, anyhow::Error> {
        let host = endpoint.address().url_host();
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

        Ok(Self::request(addrs, ports, endpoint, voice_websocket))
    }
}
