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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connected_server: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub player_display: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub player_hash: Option<String>,
    #[serde(flatten)]
    pub custom: HashMap<String, serde_json::Value>,
}
