use serde::{Deserialize, Serialize};

fn default_minecraft_client_id() -> String {
    "a17f9693-f01f-4d1d-ad12-1f179478375d".to_string()
}

/// Minecraft Configuration
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApplicationConfigMinecraft {
    pub access_token: String,
    #[serde(default = "default_minecraft_client_id")]
    pub client_id: String,
}

impl Default for ApplicationConfigMinecraft {
    fn default() -> Self {
        Self {
            access_token: String::new(),
            client_id: default_minecraft_client_id(),
        }
    }
}
