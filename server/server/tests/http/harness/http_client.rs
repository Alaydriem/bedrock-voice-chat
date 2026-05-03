//! Helpers for building reqwest clients with various TLS configurations.

use std::time::Duration;

use anyhow::Result;
use reqwest::{Certificate as ReqwestCa, Client, Identity as ReqwestIdentity};

pub struct MtlsClient;

impl MtlsClient {
    pub fn with_identity(ca_pem: &str, cert_pem: &str, key_pem: &str) -> Result<Client> {
        Self::build(ca_pem, Some((cert_pem, key_pem)))
    }

    pub fn no_identity(ca_pem: &str) -> Result<Client> {
        Self::build(ca_pem, None)
    }

    fn build(ca_pem: &str, identity: Option<(&str, &str)>) -> Result<Client> {
        let mut builder = Client::builder()
            .use_rustls_tls()
            .https_only(true)
            .add_root_certificate(ReqwestCa::from_pem(ca_pem.as_bytes())?)
            .timeout(Duration::from_secs(5));
        if let Some((cert, key)) = identity {
            let mut combined = Vec::with_capacity(cert.len() + key.len() + 1);
            combined.extend_from_slice(cert.as_bytes());
            combined.push(b'\n');
            combined.extend_from_slice(key.as_bytes());
            builder = builder.identity(ReqwestIdentity::from_pem(&combined)?);
        }
        Ok(builder.build()?)
    }
}
