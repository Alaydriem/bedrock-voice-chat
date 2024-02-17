use std::{ path::Path, time::Duration };

use tokio::fs::File;
use tokio::io::AsyncReadExt;

use reqwest::{ header::{ HeaderMap, HeaderValue }, Certificate, Client as ReqwestClient, Identity };
use anyhow::anyhow;

/// The HTTP Client for calling the API
pub(crate) struct Client {
    /// The CA Certificate of the server
    ca_cert: String,
    /// The PEM certificate of the client.
    pem: String,
}

impl Client {
    /// Creates a new client
    pub(crate) async fn new() -> Result<Self, anyhow::Error> {
        let ca = match crate::invocations::credentials::get_credential("ca".to_string()).await {
            Ok(ca) => ca,
            Err(_) => {
                return Err(anyhow!("Missing credentials."));
            }
        };

        let key = match crate::invocations::credentials::get_credential("key".to_string()).await {
            Ok(ca) => ca,
            Err(_) => {
                return Err(anyhow!("Missing credentials."));
            }
        };

        let certificate = match
            crate::invocations::credentials::get_credential("certificate".to_string()).await
        {
            Ok(ca) => ca,
            Err(_) => {
                return Err(anyhow!("Missing credentials."));
            }
        };

        let pem = format!("{}\n{}", certificate, key);

        Ok(Self {
            ca_cert: ca,
            pem: pem,
        })
    }

    /// Get's the CA Bytes from the pem
    async fn get_ca_bytes(&self) -> Result<Certificate, anyhow::Error> {
        let mut buf = self.ca_cert.as_bytes();

        match reqwest::Certificate::from_pem(&buf) {
            Ok(cert) => Ok(cert),
            Err(e) => Err(anyhow!(e.to_string())),
        }
    }

    /// Gets the identity bytes from the pem
    async fn get_identity_bytes(&self) -> Result<Identity, anyhow::Error> {
        let mut buf = self.pem.as_bytes();

        match reqwest::Identity::from_pem(&buf) {
            Ok(cert) => Ok(cert),
            Err(e) => Err(anyhow!(e.to_string())),
        }
    }

    /// Builds the reqwest client
    pub(crate) async fn get_reqwest_client(&self) -> ReqwestClient {
        let mut builder = ReqwestClient::builder()
            .use_rustls_tls()
            .timeout(Duration::new(3, 0))
            .add_root_certificate(self.get_ca_bytes().await.unwrap())
            .identity(self.get_identity_bytes().await.unwrap());

        #[cfg(debug_assertions)]
        {
            builder = builder.danger_accept_invalid_certs(true);
        }

        builder.build().unwrap()
    }
}
