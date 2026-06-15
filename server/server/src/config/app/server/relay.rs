use serde::{Deserialize, Serialize};

fn default_false() -> bool {
    false
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RelayFeature {
    // Run the discovery relay role on this server (serve the /relay/* routes).
    #[serde(default = "default_false")]
    pub enabled: bool,
    #[serde(default)]
    pub store_dsn: Option<String>,
    // Override the discovery relay this server federates against. Absent uses
    // RelayClient::DEFAULT_RELAY_URL (https://relay.bedrockvoicechat.com), which is
    // certificate-pinned. A custom URL is used as-is with pinning removed (still
    // HTTPS) for bring-your-own-relay or local testing. Every server federates for
    // cross-server voice by default; there is no opt-out.
    #[serde(default)]
    pub client_url: Option<String>,
}

impl Default for RelayFeature {
    fn default() -> Self {
        Self {
            enabled: false,
            store_dsn: None,
            client_url: None,
        }
    }
}
