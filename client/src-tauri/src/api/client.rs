use common::reqwest::Client as ReqwestClient;
use log::error;
use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;
use url::Url;

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

    pub(crate) async fn get_client(&self, fqdn: Option<&str>) -> ReqwestClient {
        let mut builder = ReqwestClient::builder()
            .use_rustls_tls()
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(5))
            .tcp_keepalive(Duration::from_secs(30))
            .pool_idle_timeout(Duration::from_secs(90))
            .add_root_certificate(self.get_ca_cert().unwrap())
            .identity(self.get_client_cert().unwrap());

        #[cfg(debug_assertions)]
        {
            builder = builder.danger_accept_invalid_certs(true);
        }

        if let Some(host) = fqdn {
            let (hostname, port) = Client::parse_host_and_port(host);

            match Client::resolve_ipv4(&hostname, port).await {
                Ok(ipv4_addr) => {
                    builder = builder.resolve(
                        &hostname,
                        std::net::SocketAddr::new(IpAddr::V4(ipv4_addr), port),
                    );
                }
                Err(e) => {
                    error!("Failed to resolve A record for {}: {}", hostname, e);
                }
            }
        }

        builder.build().unwrap()
    }

    /// Parse a URL or host string to extract hostname and port.
    /// Returns (hostname, port) where port defaults to 443 if not specified.
    fn parse_host_and_port(host: &str) -> (String, u16) {
        // Try parsing as a full URL first
        if let Ok(url) = Url::parse(host) {
            let hostname = url.host_str().unwrap_or(host).to_string();
            let port = url.port().unwrap_or(443);
            return (hostname, port);
        }

        // Fallback: handle as host:port or just host
        let host = host.replace("https://", "").replace("http://", "");
        if let Some((hostname, port_str)) = host.split_once(':') {
            let port = port_str.parse().unwrap_or(443);
            (hostname.to_string(), port)
        } else {
            (host, 443)
        }
    }

    // IPv6 Jank on Windows. Only use V4 addresses for now.
    pub(crate) async fn resolve_ipv4(hostname: &str, port: u16) -> Result<Ipv4Addr, anyhow::Error> {
        let target = format!("{}:{}", hostname, port);
        let addr = tokio::net::lookup_host(&target)
            .await?
            .find_map(|sa| match sa.ip() {
                IpAddr::V4(v4) => Some(v4),
                _ => None,
            })
            .ok_or_else(|| anyhow!("no IPv4 address resolved for {}", hostname))?;
        Ok(addr)
    }
}
