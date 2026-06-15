use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
pub struct RelayEndpoint {
    pub host: String,
    pub port: u16,
    #[serde(default)]
    pub primary: bool,
}
