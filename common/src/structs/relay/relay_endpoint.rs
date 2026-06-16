use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
pub struct RelayEndpoint {
    pub host: String,
    pub port: u16,
    #[serde(default)]
    pub primary: bool,
}
