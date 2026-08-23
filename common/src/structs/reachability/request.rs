use serde::{Deserialize, Serialize};
use std::net::IpAddr;

// Probe inputs. Lives here rather than under `crate::request`, which holds HTTP
// API request bodies; this never crosses a wire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReachabilityRequest {
    pub host: String,
    pub addrs: Vec<IpAddr>,
    pub quic_ports: Vec<u16>,
    pub https_url: String,
    // Carried rather than parsed out of `https_url` or assumed to be 443, so an
    // operator serving HTTP on another port is reported at the port actually
    // measured.
    pub https_port: u16,
    // Whether the server advertised the WebSocket voice transport, from
    // `ApiConfigResponse::voice_websocket`. False suppresses that probe rather than
    // failing it: a server which never claimed the transport would answer the ALPN
    // with a refusal, and spending a TLS handshake to be told what the config
    // already said costs the player time on the one screen that waits for it.
    pub voice_websocket: bool,
}

impl ReachabilityRequest {
    pub const DEFAULT_HTTPS_PORT: u16 = 443;

    pub fn new(
        host: String,
        addrs: Vec<IpAddr>,
        quic_ports: Vec<u16>,
        https_url: String,
        https_port: u16,
        voice_websocket: bool,
    ) -> Self {
        Self {
            host,
            addrs,
            quic_ports,
            https_url,
            https_port,
            voice_websocket,
        }
    }
}
