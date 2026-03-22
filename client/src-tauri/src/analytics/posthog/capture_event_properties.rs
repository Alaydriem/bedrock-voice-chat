use std::collections::HashMap;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct CaptureEventProperties {
    #[serde(rename = "$session_id")]
    pub session_id: String,
    #[serde(rename = "$os")]
    pub os: String,
    #[serde(rename = "$app_version")]
    pub app_version: String,
    pub is_debug: bool,
    #[serde(flatten)]
    pub custom: HashMap<String, serde_json::Value>,
}
