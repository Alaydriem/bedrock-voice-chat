use serde::{Deserialize, Serialize};

// Credentials for the zone the relay writes assigned names into.
//
// The token is the highest-value secret in the deployment: it can rewrite every
// operator's address and complete a DNS-01 challenge for any name in the zone. It
// is scoped to `Zone.DNS:Edit` on one zone and nothing else.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudflareConfig {
    pub api_token: String,
    pub zone_id: String,
}
