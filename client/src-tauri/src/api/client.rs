use common::reqwest::Client as ReqwestClient;
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

    pub(crate) fn get_client(&self) -> ReqwestClient {
        // Server domains publish both A and AAAA records, and some Windows
        // machines advertise an IPv6 stack they cannot route. Binding the local
        // socket to IPv4 keeps every connection on IPv4 while still letting the
        // connector see, and fall back across, every resolved A record.
        let mut builder = ReqwestClient::builder()
            .use_rustls_tls()
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(5))
            .tcp_keepalive(Duration::from_secs(30))
            .pool_idle_timeout(Duration::from_secs(90))
            .local_address(IpAddr::V4(Ipv4Addr::UNSPECIFIED))
            .add_root_certificate(self.get_ca_cert().unwrap())
            .identity(self.get_client_cert().unwrap());

        #[cfg(debug_assertions)]
        {
            builder = builder.danger_accept_invalid_certs(true);
        }

        builder.build().unwrap()
    }
}
