pub(crate) mod commands;
use log::error;
mod channel;
mod client;

use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client as ReqwestClient, StatusCode,
};
use std::error::Error;

#[derive(Debug, Clone)]
pub(crate) struct Api {
    endpoint: String,
    client: client::Client,
}

impl Api {
    pub fn new(endpoint: String, ca_cert: String, pem: String) -> Self {
        Self {
            endpoint,
            client: client::Client::new(ca_cert, pem),
        }
    }

    async fn get_client(&self, fqdn: Option<&str>) -> ReqwestClient {
        self.client.get_client(fqdn).await
    }

    pub(crate) async fn ping(&self) -> Result<(), bool> {
        let client = self.get_client(Some(self.endpoint.as_str())).await;

        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        headers.insert("Accept", HeaderValue::from_static("application/json"));

        // Reconstruct full URL with resolved IP address
        let url = format!("{}/api/ping", self.endpoint);

        match client.get(url).headers(headers).send().await {
            Ok(response) => match response.status() {
                StatusCode::OK => Ok(()),
                _ => Err(false),
            },
            Err(e) => {
                error!("Unable to connect to BVC Server: {} {}", self.endpoint, e);
                let mut source = e.source();
                while let Some(cause) = source {
                    error!("Caused by: {}", cause);
                    source = cause.source();
                }
                Err(false)
            }
        }
    }
}
