pub(crate) mod admin;
pub(crate) mod audio_library;
pub(crate) mod chat;
pub(crate) mod commands;
use common::request::LinkJavaIdentityRequest;
use common::response::ApiConfigResponse;
use common::response::LinkJavaIdentityResponse;
use log::{error, warn};
mod channel;
mod circuit_breaker;
mod client;
mod credential_verdict;
mod fetch_cache;
mod gamerpic;

pub use fetch_cache::FetchCache;

pub(crate) use circuit_breaker::EndpointBreaker;
pub(crate) use credential_verdict::CredentialVerdict;

use common::reqwest::{
    Client as ReqwestClient, StatusCode,
    header::{HeaderMap, HeaderValue},
};
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;

// Connection-establishment failures are transient often enough (proxy dial,
// handshake timeout) that a couple of quick retries resolve them; anything
// still failing after that is a real outage for the breaker to handle.
const MAX_SEND_ATTEMPTS: u32 = 3;
const RETRY_BASE_DELAY: Duration = Duration::from_millis(250);

#[derive(Debug, Clone)]
pub(crate) struct Api {
    endpoint: String,
    client: client::Client,
    // Shared, not copied: this Api is pooled per server and long-lived, so a
    // verdict captured at construction would freeze. The cell also bridges the
    // async/sync boundary, since get_client cannot await a probe.
    family_preference: Arc<common::net::FamilyPreferenceCell>,
    // Cloned out of the pool per call, so the handle has to be one every clone shares. A
    // cache created per clone would never see a second hit. `moka::future::Cache` is
    // internally shared, so deriving Clone gives that for free.
    config: FetchCache<ApiConfigResponse>,
}

impl Api {
    // Long enough to cover one launch, where the age gate, the port and spatial-audio
    // refresh, the bedrock hints and the connect all read this document. Far shorter than
    // the reachability probe's window, because this one decides which port to dial.
    const CONFIG_TTL: Duration = Duration::from_secs(30);
    const CONFIG_CAPACITY: u64 = 16;

    pub fn new(
        endpoint: String,
        ca_cert: String,
        pem: String,
        family_preference: Arc<common::net::FamilyPreferenceCell>,
    ) -> Self {
        Self {
            endpoint,
            client: client::Client::new(ca_cert, pem),
            family_preference,
            config: FetchCache::new(Self::CONFIG_TTL, Self::CONFIG_CAPACITY),
        }
    }

    fn get_client(&self) -> ReqwestClient {
        self.client.get_client(self.family_preference.get())
    }

    /// Send a prepared request through this endpoint's circuit breaker. While the
    /// breaker is open the request short-circuits without touching the network.
    /// Connection-establishment failures are retried with backoff — the request
    /// never reached the server, so a second attempt cannot duplicate a
    /// non-idempotent action. The logical outcome (reachable response or final
    /// transport failure) is recorded so the breaker can open or close.
    async fn send(
        &self,
        request: common::reqwest::RequestBuilder,
    ) -> Result<common::reqwest::Response, circuit_breaker::SendError> {
        let breaker = circuit_breaker::EndpointBreaker::for_endpoint(&self.endpoint);
        if !breaker.allow() {
            return Err(circuit_breaker::SendError::Open);
        }

        let mut current = request;
        let mut attempt = 1u32;
        loop {
            // A streaming body cannot be cloned; such a request gets no retry.
            let retry = current.try_clone();
            match current.send().await {
                Ok(response) => {
                    breaker.on_success();
                    return Ok(response);
                }
                Err(e) if attempt < MAX_SEND_ATTEMPTS && e.is_connect() && retry.is_some() => {
                    warn!(
                        "Attempt {}/{} to {} failed; retrying: {}",
                        attempt,
                        MAX_SEND_ATTEMPTS,
                        self.endpoint,
                        Self::error_chain(&e)
                    );
                    tokio::time::sleep(RETRY_BASE_DELAY * attempt).await;
                    current = retry.unwrap();
                    attempt += 1;
                }
                Err(e) => {
                    error!(
                        "Request to {} failed after {} attempt(s): {}",
                        self.endpoint,
                        attempt,
                        Self::error_chain(&e)
                    );
                    if breaker.on_transport_failure() {
                        warn!(
                            "Repeated connection failures to {}; backing off further requests",
                            self.endpoint
                        );
                    }
                    return Err(circuit_breaker::SendError::Transport(e));
                }
            }
        }
    }

    /// Render a reqwest error with its full `source()` chain. The top-level
    /// Display ("error sending request for url (...)") omits the underlying
    /// cause — connect timeout, TLS failure, reset — which is the part that
    /// identifies the fault.
    fn error_chain(e: &common::reqwest::Error) -> String {
        let mut out = e.to_string();
        let mut source = e.source();
        while let Some(cause) = source {
            out.push_str(": ");
            out.push_str(&cause.to_string());
            source = cause.source();
        }
        out
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
    pub(crate) async fn resolve_bedrock_connection_hints(&self) -> Option<String> {
        match self.get_config().await {
            Ok(config) => config
                .bedrock
                .transfer_port
                .map(|port| self.transfer_relay_address(port)),
            Err(e) => {
                error!("Failed to fetch /api/config for bedrock connection hints: {}", e);
                None
            }
        }
    }

    pub(crate) fn get_reqwest_client(&self) -> ReqwestClient {
        self.client.get_client(self.family_preference.get())
    }

    pub(crate) async fn ping(&self) -> Result<(), bool> {
        let client = self.get_client();

        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        headers.insert("Accept", HeaderValue::from_static("application/json"));

        // Reconstruct full URL with resolved IP address
        let url = format!("{}/api/ping", self.endpoint);

        match self.send(client.get(url).headers(headers)).await {
            Ok(response) => match response.status() {
                StatusCode::OK => Ok(()),
                _ => Err(false),
            },
            Err(circuit_breaker::SendError::Open) => Err(false),
            Err(circuit_breaker::SendError::Transport(e)) => {
                error!("Unable to connect to BVC Server: {} {}", self.endpoint, e);
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
        let client = self.get_client();

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

        match self
            .send(client.post(url).headers(headers).json(&payload))
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
            Err(circuit_breaker::SendError::Open) => {
                Err("Server temporarily unreachable; backing off".to_string())
            }
            Err(circuit_breaker::SendError::Transport(e)) => {
                error!("Failed to link Java identity: {}", e);
                Err(format!("Connection failed: {}", e))
            }
        }
    }

    /// Trade this client's mTLS identity for a single-use WebSocket ticket.
    ///
    /// The webview can present neither a client certificate nor a request header, so it
    /// cannot open an authenticated socket by itself. This call happens here, where the
    /// certificate is, and hands back something the webview can offer as a subprotocol.
    ///
    /// Never cached: a ticket is single-use and expires within the minute, so every
    /// reconnect wants a fresh one and holding one only buys a failed handshake.
    pub(crate) async fn websocket_ticket(
        &self,
    ) -> Result<common::response::websocket::WebsocketTicketResponse, String> {
        let client = self.get_client();

        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        headers.insert("Accept", HeaderValue::from_static("application/json"));

        let url = format!("{}/api/websocket/ticket", self.endpoint);

        let response = match self.send(client.post(url).headers(headers)).await {
            Ok(response) => response,
            // The breaker's own error carries no message worth relaying, and the caller only
            // needs to know it did not arrive so it can try again in a few seconds.
            Err(_) => return Err("Ticket request did not reach the server".to_string()),
        };

        if response.status() != StatusCode::OK {
            return Err(format!("Ticket request returned {}", response.status()));
        }

        let body = response
            .text()
            .await
            .map_err(|e| format!("Failed to read ticket response: {}", e))?;

        serde_json::from_str(&body).map_err(|e| format!("Failed to parse ticket response: {}", e))
    }

    /// `/api/config`, from this window's copy when there is one.
    ///
    /// Several callers read different fields of this document within a second of each other
    /// on the screen the user is waiting on: the age gate, the port and spatial-audio
    /// refresh, the bedrock connection hints, and the connect path's candidate planning.
    pub(crate) async fn get_config(&self) -> Result<ApiConfigResponse, String> {
        let endpoint = self.endpoint.clone();
        self.config
            .get_or_fetch(&endpoint, || self.fetch_config())
            .await
    }

    /// Drop this server's cached config so the next read goes to the network.
    pub(crate) async fn invalidate_config(&self) {
        self.config.invalidate(&self.endpoint).await;
    }

    async fn fetch_config(&self) -> Result<ApiConfigResponse, String> {
        let client = self.get_client();

        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        headers.insert("Accept", HeaderValue::from_static("application/json"));

        let url = format!("{}/api/config", self.endpoint);

        match self.send(client.get(url).headers(headers)).await {
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
                        warn!(
                            "Config from {} parsed via legacy fallback; server predates bedrock/protocol fields",
                            self.endpoint
                        );
                        return Ok(ApiConfigResponse {
                            status: legacy.status,
                            client_id: legacy.client_id,
                            protocol_version: String::new(),
                            quic_port: 0,
                            quic_ports: Vec::new(),
                            voice_websocket: false,
                            spatial_audio: Default::default(),
                            bedrock: Default::default(),
                            age: Default::default(),
                            recording: Default::default(),
                            chat: Default::default(),
                            capacity: Default::default(),
                        });
                    }

                    Err("Failed to parse config response".to_string())
                }
                status => Err(format!("Server returned status: {}", status)),
            },
            Err(circuit_breaker::SendError::Open) => {
                Err("Server temporarily unreachable; backing off".to_string())
            }
            Err(circuit_breaker::SendError::Transport(e)) => {
                error!(
                    "Unable to get config from BVC Server: {} {}",
                    self.endpoint, e
                );
                Err(format!("Connection failed: {}", e))
            }
        }
    }
}
