use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChangeNetworkStreamEvent {
    pub server: String,
    pub socket: String,
    pub name: String,
    pub ca_cert: String,
    pub cert: String,
    pub key: String
}
