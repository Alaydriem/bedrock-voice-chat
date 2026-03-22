use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct MicrosoftAuthCodeAndUrlResponse {
    pub url: String,
    pub state: String,
}
