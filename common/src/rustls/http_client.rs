use std::path::Path;
use std::time::Duration;

use reqwest::{Certificate, Client, Identity};

pub struct MtlsHttpClient {
    client: Client,
}

impl MtlsHttpClient {
    pub async fn new<A: AsRef<Path>, B: AsRef<Path>, C: AsRef<Path>>(
        ca_cert_path: A,
        cert_path: B,
        key_path: C,
    ) -> Result<Self, anyhow::Error> {
        let ca_pem = tokio::fs::read(ca_cert_path.as_ref()).await?;
        let ca = Certificate::from_pem(&ca_pem)?;

        let cert_pem = tokio::fs::read_to_string(cert_path.as_ref()).await?;
        let key_pem = tokio::fs::read_to_string(key_path.as_ref()).await?;
        let combined = format!("{}\n{}", cert_pem.trim(), key_pem.trim());
        let identity = Identity::from_pem(combined.as_bytes())?;

        let client = Client::builder()
            .use_rustls_tls()
            .timeout(Duration::from_secs(10))
            .add_root_certificate(ca)
            .identity(identity)
            .build()?;

        Ok(Self { client })
    }

    pub fn client(&self) -> &Client {
        &self.client
    }
}
