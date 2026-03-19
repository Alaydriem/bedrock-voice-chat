use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Meridian {
    pub url: String,
    pub api_key: String,
    pub instance_id: u16,
    #[serde(default)]
    pub host: Option<String>,
    pub backend: String,
}
