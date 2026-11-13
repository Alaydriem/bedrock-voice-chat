use std::time::Duration;

use reqwest::Client;
use tokio::sync::OnceCell;

use super::{SrvLookup, SrvWire};
use crate::rustls::BvcClientTls;

/// Looks up an SRV record over DNS-over-HTTPS (RFC 8484) with the strict public-roots client.
pub struct DohLookup {
    url: String,
    timeout: Duration,
    client: OnceCell<Client>,
}

impl DohLookup {
    const CONTENT_TYPE: &'static str = "application/dns-message";

    pub fn new(url: String, timeout: Duration) -> Self {
        Self {
            url,
            timeout,
            client: OnceCell::new(),
        }
    }

    pub async fn lookup(&self, host: &str) -> SrvLookup {
        match self.exchange(host).await {
            Ok(lookup) => lookup,
            Err(reason) => SrvLookup::Failed(reason),
        }
    }

    async fn exchange(&self, host: &str) -> Result<SrvLookup, String> {
        let client = self
            .client
            .get_or_try_init(|| async {
                BvcClientTls::http_client_builder(None, None)
                    .map_err(|error| error.to_string())?
                    .timeout(self.timeout)
                    .build()
                    .map_err(|error| error.to_string())
            })
            .await?;

        let body = SrvWire::encode(&SrvWire::query(host, true)?)?;
        let response = client
            .post(&self.url)
            .header("content-type", Self::CONTENT_TYPE)
            .header("accept", Self::CONTENT_TYPE)
            .body(body)
            .send()
            .await
            .map_err(|error| error.to_string())?
            .error_for_status()
            .map_err(|error| error.to_string())?;
        let bytes = response.bytes().await.map_err(|error| error.to_string())?;

        Ok(SrvWire::classify(&SrvWire::decode(&bytes)?))
    }
}
