use serde::{Deserialize, Serialize};

fn default_directory() -> String {
    "https://acme-v02.api.letsencrypt.org/directory".to_string()
}

// Credentials for obtaining the registry's own certificate.
//
// A separate token from the one that writes assigned names is permitted but not
// required: one token with access to both zones is the arrangement this is built for,
// and the zone is discovered from the hostname rather than configured.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AcmeConfig {
    pub email: String,
    pub api_token: String,
    // Overridden only to point at a staging directory during development. The default
    // is production, because a registry serving a staging certificate fails as a TLS
    // warning in somebody's browser rather than as anything that names the cause.
    #[serde(default = "default_directory")]
    pub directory: String,
}
