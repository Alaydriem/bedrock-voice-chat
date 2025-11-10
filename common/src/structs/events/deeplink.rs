use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../../client/src/js/bindings/")]
#[serde(rename_all = "camelCase")]
pub struct DeepLink {
    pub url: String,
    pub timestamp: i64,
}

impl DeepLink {
    /// Create a new DeepLink event
    pub fn new(url: String) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        Self {
            url,
            timestamp,
        }
    }
}
