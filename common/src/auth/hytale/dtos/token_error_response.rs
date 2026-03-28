use serde::Deserialize;

/// Error response from token endpoint
#[derive(Deserialize)]
pub(crate) struct TokenErrorResponse {
    pub error: String,
    #[serde(default)]
    pub error_description: Option<String>,
}
