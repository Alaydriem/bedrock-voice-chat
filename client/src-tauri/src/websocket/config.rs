use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct WebSocketConfig {
    pub enabled: bool,
    pub localhost_only: bool,
    pub port: u16,
    pub key: String,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            // Loopback on desktop; a phone has nothing local to serve.
            localhost_only: !cfg!(mobile),
            port: 9595,
            key: String::new(),
        }
    }
}
