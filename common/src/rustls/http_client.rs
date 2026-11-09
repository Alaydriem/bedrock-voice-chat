use std::path::Path;
use std::time::Duration;

use reqwest::Client;

use super::{BvcClientTls, VerifierMode};

pub struct MtlsHttpClient {
    client: Client,
}

impl MtlsHttpClient {
    pub async fn new<A: AsRef<Path>, B: AsRef<Path>, C: AsRef<Path>>(
        ca_cert_path: A,
        cert_path: B,
        key_path: C,
    ) -> Result<Self, anyhow::Error> {
        let ca_pem = tokio::fs::read_to_string(ca_cert_path.as_ref()).await?;

        let cert_pem = tokio::fs::read_to_string(cert_path.as_ref()).await?;
        let key_pem = tokio::fs::read_to_string(key_path.as_ref()).await?;
        let combined = format!("{}\n{}", cert_pem.trim(), key_pem.trim());
        let identity = BvcClientTls::identity_from_pem(&combined)?;

        let client = BvcClientTls::http_client_builder(
            Some(&ca_pem),
            Some(identity),
            VerifierMode::Strict,
        )?
            .timeout(Duration::from_secs(10))
            .build()?;

        Ok(Self { client })
    }

    pub fn client(&self) -> &Client {
        &self.client
    }
}
