use super::{EndpointSource, ServerAddress};

/// A server address with the port to reach it on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerEndpoint {
    address: ServerAddress,
    port: u16,
    source: EndpointSource,
}

impl ServerEndpoint {
    pub fn new(address: ServerAddress, port: u16, source: EndpointSource) -> Self {
        Self {
            address,
            port,
            source,
        }
    }

    pub fn address(&self) -> &ServerAddress {
        &self.address
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn source(&self) -> EndpointSource {
        self.source
    }

    pub fn base_url(&self) -> String {
        format!("https://{}:{}", self.address.url_host(), self.port)
    }

    pub fn wss_url(&self, path: &str) -> String {
        format!("wss://{}:{}{}", self.address.url_host(), self.port, path)
    }

    // The TLS name is always the typed host, never an SRV target.
    pub fn tls_name(&self) -> &str {
        self.address.host()
    }
}
