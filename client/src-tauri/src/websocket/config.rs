use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct WebSocketConfig {
    /// Retained so a config stored before the server became always-on still deserializes.
    /// Read by the settings manager's one-time migration and by nothing else.
    pub enabled: bool,
    /// Retained for the same migration. It no longer decides the bind address, because on
    /// mobile it was forced rather than chosen and cannot be read as a preference.
    pub localhost_only: bool,
    /// Whether the operator-facing listener answers on every interface.
    pub allow_external: bool,
    pub port: u16,
    pub key: String,
}

impl WebSocketConfig {
    /// Where the operator-facing listener binds.
    ///
    /// Loopback unless external access was asked for. The internal listener is separate and
    /// always loopback, so this decides third-party reach only.
    pub fn bind_host(&self) -> &'static str {
        if self.allow_external {
            "0.0.0.0"
        } else {
            "127.0.0.1"
        }
    }
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            localhost_only: true,
            allow_external: false,
            port: 9595,
            key: String::new(),
        }
    }
}
