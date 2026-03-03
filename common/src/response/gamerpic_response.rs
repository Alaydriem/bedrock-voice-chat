use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct GamerpicResponse {
    pub gamertag: String,
    pub game: String,
    pub gamerpic: Option<String>,
}
