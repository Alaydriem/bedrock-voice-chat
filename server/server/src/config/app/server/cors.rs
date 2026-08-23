use serde::{Deserialize, Serialize};

fn default_allow_credentials() -> bool {
    true
}

#[derive(Serialize, Deserialize, Debug, Clone, schemars::JsonSchema)]
pub struct Cors {
    // Origins permitted by CORS. An empty list allows all origins (the historical
    // default, harmless for the native mTLS client); a non-empty list restricts
    // access to exactly those origins.
    #[serde(default)]
    pub allowed_origins: Vec<String>,
    #[serde(default = "default_allow_credentials")]
    pub allow_credentials: bool,
}

impl Default for Cors {
    fn default() -> Self {
        Self {
            allowed_origins: Vec::new(),
            allow_credentials: default_allow_credentials(),
        }
    }
}
