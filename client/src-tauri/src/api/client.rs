use common::reqwest::Client as ReqwestClient;
use common::structs::reachability::AddressFamilyPreference;
use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;

use anyhow::anyhow;
use tauri_plugin_http::reqwest::{self, Certificate, Identity};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct Client {
    ca_cert: String,
    pem: String,
}

impl Client {
    pub fn new(ca_cert: String, pem: String) -> Self {
        Self { ca_cert, pem }
    }

    fn get_ca_cert(&self) -> Result<Certificate, anyhow::Error> {
        let buf = self.ca_cert.as_bytes();

        match reqwest::Certificate::from_pem(&buf) {
            Ok(cert) => Ok(cert),
            Err(e) => Err(anyhow!(e.to_string())),
        }
    }

    fn get_client_cert(&self) -> Result<Identity, anyhow::Error> {
        let buf = self.pem.as_bytes();

        match reqwest::Identity::from_pem(&buf) {
            Ok(cert) => Ok(cert),
            Err(e) => Err(anyhow!(e.to_string())),
        }
    }

    // Under PreferIpv4 the local socket is pinned to IPv4, which is what every
    // released version does: server domains publish both A and AAAA records, and
    // some Windows machines advertise an IPv6 stack they cannot route, so pinning
    // keeps every connection on IPv4 while still letting the connector fall back
    // across every resolved A record.
    //
    // Under PreferIpv6 the pin is omitted, which hands the choice to hyper-util's
    // Happy Eyeballs: IPv6 first, IPv4 racing 300ms behind. The pin is lifted only
    // for a host where a probe has already seen IPv6 answering, so that fallback
    // delay is the exception rather than something every request pays.
    pub(crate) fn get_client(&self, preference: AddressFamilyPreference) -> ReqwestClient {
        let mut builder = ReqwestClient::builder()
            .use_rustls_tls()
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(5))
            .tcp_keepalive(Duration::from_secs(30))
            .pool_idle_timeout(Duration::from_secs(90))
            .add_root_certificate(self.get_ca_cert().unwrap())
            .identity(self.get_client_cert().unwrap());

        if matches!(preference, AddressFamilyPreference::PreferIpv4) {
            builder = builder.local_address(IpAddr::V4(Ipv4Addr::UNSPECIFIED));
        }

        #[cfg(debug_assertions)]
        {
            builder = builder.danger_accept_invalid_certs(true);
        }

        builder.build().unwrap()
    }
}
