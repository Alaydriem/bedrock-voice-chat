use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct EventProperties {
    pub server_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname_sha: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_duration_secs: Option<u64>,
}
