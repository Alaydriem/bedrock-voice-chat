pub(crate) mod audio_library;
pub(crate) mod commands;
use common::request::LinkJavaIdentityRequest;
use common::response::ApiConfigResponse;
use common::response::LinkJavaIdentityResponse;
use log::error;
mod channel;
mod client;
mod gamerpic;

use common::reqwest::{
    Client as ReqwestClient, StatusCode,
    header::{HeaderMap, HeaderValue},
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

    pub(crate) fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Build the connected server's bedrock transfer relay address (`host:port`)
    /// from this client's endpoint and the transfer port advertised by
    /// `/api/config`. The host is the server's hostname without scheme or HTTPS
    /// port; the supplied port is the bedrock transfer port.
    pub(crate) fn transfer_relay_address(&self, transfer_port: u16) -> String {
        let host = url::Url::parse(&self.endpoint)
            .ok()
            .and_then(|u| u.host_str().map(|s| s.to_string()))
            .unwrap_or_else(|| {
                self.endpoint
                    .replace("https://", "")
                    .replace("http://", "")
                    .split(':')
                    .next()
                    .unwrap_or(&self.endpoint)
                    .to_string()
            });
        format!("{}:{}", host, transfer_port)
    }

    /// Fetch `/api/config` and derive the bedrock connection hints for the
    /// connection menu: the server transfer relay address (`host:port`, present
    /// only when the relay is enabled) and whether the server's DNS override is
    /// running. Both fall back to absent/false on a config fetch failure.
    pub(crate) async fn resolve_bedrock_connection_hints(&self) -> (Option<String>, bool) {
        match self.get_config().await {
            Ok(config) => (
                config
                    .bedrock
                    .transfer_port
                    .map(|port| self.transfer_relay_address(port)),
                config.bedrock.dns_enabled,
            ),
            Err(e) => {
                error!("Failed to fetch /api/config for bedrock connection hints: {}", e);
                (None, false)
            }
        }
    }

    pub(crate) async fn get_reqwest_client(&self) -> ReqwestClient {
        self.client.get_client(Some(self.endpoint.as_str())).await
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

    pub(crate) async fn link_java_identity(
        &self,
        code: String,
        redirect_uri: String,
        client_id: String,
        gamertag: String,
    ) -> Result<LinkJavaIdentityResponse, String> {
        let client = self.get_client(Some(self.endpoint.as_str())).await;

        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        headers.insert("Accept", HeaderValue::from_static("application/json"));

        let url = format!("{}/api/auth/link-java", self.endpoint);
        let payload = LinkJavaIdentityRequest {
            code,
            redirect_uri,
            client_id,
            gamertag,
        };

        match client
            .post(url)
            .headers(headers)
            .json(&payload)
            .send()
            .await
        {
            Ok(response) => match response.status() {
                StatusCode::OK => {
                    let body = response
                        .text()
                        .await
                        .map_err(|e| format!("Failed to read response: {}", e))?;
                    serde_json::from_str::<LinkJavaIdentityResponse>(&body)
                        .map_err(|e| format!("Failed to parse response: {}", e))
                }
                status => Err(format!("Server returned status: {}", status)),
            },
            Err(e) => {
                error!("Failed to link Java identity: {}", e);
                Err(format!("Connection failed: {}", e))
            }
        }
    }

    pub(crate) async fn get_config(&self) -> Result<ApiConfigResponse, String> {
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

                    if let Ok(config) = serde_json::from_str::<ApiConfigResponse>(&body) {
                        return Ok(config);
                    }

                    #[derive(serde::Deserialize)]
                    struct LegacyApiConfig {
                        status: String,
                        client_id: String,
                    }

                    if let Ok(legacy) = serde_json::from_str::<LegacyApiConfig>(&body) {
                        return Ok(ApiConfigResponse {
                            status: legacy.status,
                            client_id: legacy.client_id,
                            protocol_version: String::new(),
                            quic_port: 0,
                            spatial_audio: Default::default(),
                            bedrock: Default::default(),
                            age: Default::default(),
                        });
                    }

                    Err("Failed to parse config response".to_string())
                }
                status => Err(format!("Server returned status: {}", status)),
            },
            Err(e) => {
                error!(
                    "Unable to get config from BVC Server: {} {}",
                    self.endpoint, e
                );
                Err(format!("Connection failed: {}", e))
            }
        }
    }
}
