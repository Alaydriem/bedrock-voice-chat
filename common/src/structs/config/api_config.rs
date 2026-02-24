use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ApiConfig {
    pub status: String,
    pub client_id: String,
    pub protocol_version: String,
}
