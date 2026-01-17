use log::error;
use reqwest::Client as ReqwestClient;
use std::net::Ipv4Addr;
use std::time::Duration;
use trust_dns_resolver::config::{ResolverConfig, ResolverOpts};
use trust_dns_resolver::TokioAsyncResolver;
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
            .timeout(Duration::new(3, 0))
            .add_root_certificate(self.get_ca_cert().unwrap())
            .identity(self.get_client_cert().unwrap());

        #[cfg(debug_assertions)]
        {
            builder = builder.danger_accept_invalid_certs(true);
        }

        // If fqdn is provided, resolve IPv4 manually
        if let Some(host) = fqdn {
            // Parse URL to extract hostname and port
            let (hostname, port) = Client::parse_host_and_port(host);

            match Client::resolve_ipv4(&hostname).await {
                Ok(ipv4_addr) => {
                    builder = builder.resolve(
                        &hostname,
                        std::net::SocketAddr::new(std::net::IpAddr::V4(ipv4_addr), port),
                    );
                }
                Err(e) => {
                    error!("Failed to resolve A record for {}: {}", hostname, e);
                    // You can choose whether to continue without resolver or propagate the error.
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
    pub(crate) async fn resolve_ipv4(hostname: &str) -> Result<Ipv4Addr, Box<dyn std::error::Error>> {
        let resolver =
            TokioAsyncResolver::tokio(ResolverConfig::cloudflare(), ResolverOpts::default());
        let response = resolver.ipv4_lookup(hostname).await?;
        let address = response.iter().next().expect("no addresses returned!");

        Ok(Ipv4Addr::new(
            address.octets()[0],
            address.octets()[1],
            address.octets()[2],
            address.octets()[3],
        ))
    }
}
