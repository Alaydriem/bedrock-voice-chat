use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApplicationConfigMeridian {
    pub url: String,
    pub api_key: String,
    pub instance_id: u16,
}
