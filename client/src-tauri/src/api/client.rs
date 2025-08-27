use log::error;
use reqwest::Client as ReqwestClient;
use std::net::Ipv4Addr;
use std::time::Duration;
use trust_dns_resolver::config::{ResolverConfig, ResolverOpts};
use trust_dns_resolver::TokioAsyncResolver;

use anyhow::anyhow;
use tauri_plugin_http::reqwest::{self, Certificate, Identity};

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
            match Client::resolve_ipv4(host).await {
                Ok(ipv4_addr) => {
                    builder = builder.resolve(
                        host,
                        std::net::SocketAddr::new(std::net::IpAddr::V4(ipv4_addr), 443),
                    );
                }
                Err(e) => {
                    error!("Failed to resolve A record for {}: {}", host, e);
                    // You can choose whether to continue without resolver or propagate the error.
                }
            }
        }

        builder.build().unwrap()
    }

    // IPv6 Jank on Windows. Only use V4 addresses for now.
    pub(crate) async fn resolve_ipv4(host: &str) -> Result<Ipv4Addr, Box<dyn std::error::Error>> {
        let host = host.replace("https://", "");
        let resolver =
            TokioAsyncResolver::tokio(ResolverConfig::cloudflare(), ResolverOpts::default());
        let response = resolver.ipv4_lookup(host).await?;
        let address = response.iter().next().expect("no addresses returned!");

        Ok(Ipv4Addr::new(
            address.octets()[0],
            address.octets()[1],
            address.octets()[2],
            address.octets()[3],
        ))
    }
}
