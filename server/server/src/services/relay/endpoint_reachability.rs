use common::structs::relay::RelayEndpoint;

use super::registry::EndpointReachability;

// HTTPS port the relay probes for the endpoint-control proof. The proof route
// (`/relay/proof/...`) is served on the registrant server's HTTP listener.
const DEFAULT_PROOF_HTTP_PORT: u16 = 443;

// `EndpointReachability` over HTTPS: confirms a registrant controls the endpoint
// it claims by fetching `/relay/proof/<nonce>` from that endpoint and checking
// the served body equals the nonce. Performs an outbound HTTPS request to the
// registrant on `proof_port`.
pub struct HttpEndpointReachability {
    http: reqwest::Client,
    proof_port: u16,
}

impl HttpEndpointReachability {
    pub fn new(port: u16) -> Self {
        Self {
            http: reqwest::Client::new(),
            proof_port: port,
        }
    }
}

impl Default for HttpEndpointReachability {
    fn default() -> Self {
        Self::new(DEFAULT_PROOF_HTTP_PORT)
    }
}

#[async_trait::async_trait]
impl EndpointReachability for HttpEndpointReachability {
    async fn serves_nonce(&self, endpoint: &RelayEndpoint, nonce: &str) -> bool {
        let url = format!(
            "https://{}:{}/relay/proof/{}",
            endpoint.host, self.proof_port, nonce
        );
        match self.http.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => match resp.text().await {
                Ok(body) => body.trim() == nonce,
                Err(_) => false,
            },
            _ => false,
        }
    }
}
