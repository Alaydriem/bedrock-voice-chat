use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct AppInfo {
    pub app_version: String,
    pub protocol_version: String,
    pub build_commit: String,
    pub build_variant: String,
    pub build_number: String,
}
