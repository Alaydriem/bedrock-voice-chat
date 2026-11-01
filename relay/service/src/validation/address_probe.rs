use std::net::SocketAddr;
use std::time::Duration;

use reqwest::Certificate;

pub struct LiveAddressProbe {
    // When set, only these roots are trusted and the platform's are not.
    roots: Option<Vec<Certificate>>,
}

impl LiveAddressProbe {
    pub const DEFAULT_PORT: u16 = 443;
    const TIMEOUT: Duration = Duration::from_secs(10);
    const PATH: &'static str = "/health/enrollment-nonce";

    pub fn new() -> Self {
        Self { roots: None }
    }

    // A probe that trusts only the certificates in `roots_pem`. The play zone's server
    // certificates chain to a root that is never added to a general store, so the
    // platform's roots would reject them and must not be accepted in its place.
    pub fn with_roots(roots_pem: &str) -> Result<Self, reqwest::Error> {
        Ok(Self {
            roots: Some(Certificate::from_pem_bundle(roots_pem.as_bytes())?),
        })
    }

    pub async fn serves_nonce(&self, name: &str, address: &str, port: u16, nonce: &str) -> bool {
        let Ok(ip) = address.parse::<std::net::IpAddr>() else {
            return false;
        };

        // `resolve` keeps the certificate validated against the name, not the address.
        let mut builder = reqwest::Client::builder()
            .timeout(Self::TIMEOUT)
            .resolve(name, SocketAddr::new(ip, port));
        if let Some(roots) = &self.roots {
            builder = builder.tls_built_in_root_certs(false);
            for root in roots {
                builder = builder.add_root_certificate(root.clone());
            }
        }
        let Ok(client) = builder.build() else {
            return false;
        };

        let url = format!("https://{name}:{port}{}", Self::PATH);
        match client.get(&url).send().await {
            Ok(response) => response
                .text()
                .await
                .map(|body| body.trim() == nonce)
                .unwrap_or(false),
            Err(_) => false,
        }
    }
}

impl Default for LiveAddressProbe {
    fn default() -> Self {
        Self::new()
    }
}

pub enum AddressProbe {
    Live(LiveAddressProbe),
    Fixed(bool),
    For(String),
    ForPort(String, u16, String),
}

impl AddressProbe {
    pub async fn serves_nonce(&self, name: &str, address: &str, port: u16, nonce: &str) -> bool {
        match self {
            Self::Live(probe) => probe.serves_nonce(name, address, port, nonce).await,
            Self::Fixed(answer) => *answer,
            Self::For(expected) => expected == name,
            Self::ForPort(expected, expected_port, expected_nonce) => {
                expected == name && *expected_port == port && expected_nonce == nonce
            }
        }
    }
}
