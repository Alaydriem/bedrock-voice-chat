pub(crate) mod commands;
use common::structs::config::ApiConfig;
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

    pub(crate) async fn get_config(&self) -> Result<ApiConfig, String> {
        let client = self.get_client(Some(self.endpoint.as_str())).await;

        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        headers.insert("Accept", HeaderValue::from_static("application/json"));

        let url = format!("{}/api/config", self.endpoint);

        match client.get(url).headers(headers).send().await {
            Ok(response) => match response.status() {
                StatusCode::OK => {
                    let body = response
                        .text()
                        .await
                        .map_err(|e| format!("Failed to read response: {}", e))?;

                    // Try to parse as new ApiConfig with protocol_version
                    if let Ok(config) = serde_json::from_str::<ApiConfig>(&body) {
                        return Ok(config);
                    }

                    // Try to parse as legacy ApiConfig (without protocol_version)
                    #[derive(serde::Deserialize)]
                    struct LegacyApiConfig {
                        status: String,
                        client_id: String,
                    }

                    if let Ok(legacy) = serde_json::from_str::<LegacyApiConfig>(&body) {
                        // Return ApiConfig with empty protocol_version to indicate outdated server
                        return Ok(ApiConfig {
                            status: legacy.status,
                            client_id: legacy.client_id,
                            protocol_version: String::new(), // Empty = outdated server
                        });
                    }

                    Err("Failed to parse config response".to_string())
                }
                status => Err(format!("Server returned status: {}", status)),
            },
            Err(e) => {
                error!("Unable to get config from BVC Server: {} {}", self.endpoint, e);
                Err(format!("Connection failed: {}", e))
            }
        }
    }
}
