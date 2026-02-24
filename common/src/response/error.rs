use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Structured API error type shared between server and client.
/// Server serializes this as the response body with the appropriate HTTP status.
/// Client deserializes it from error responses and uses `Display` for user-facing messages.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
#[serde(tag = "error", content = "detail")]
pub enum ApiError {
    #[serde(rename = "file_too_large")]
    FileTooLarge { max_bytes: u64 },

    #[serde(rename = "audio_too_long")]
    AudioTooLong { max_duration_ms: u64 },

    #[serde(rename = "invalid_format")]
    InvalidFormat,

    #[serde(rename = "parse_failed")]
    ParseFailed,

    #[serde(rename = "not_found")]
    NotFound,

    #[serde(rename = "forbidden")]
    Forbidden,

    #[serde(rename = "auth_failed")]
    AuthFailed,

    #[serde(rename = "duplicate")]
    Duplicate,

    #[serde(rename = "internal")]
    Internal,
}

impl ApiError {
    /// Returns the HTTP status code this error should map to.
    pub fn status_code(&self) -> u16 {
        match self {
            ApiError::FileTooLarge { .. } => 413,
            ApiError::AudioTooLong { .. } => 422,
            ApiError::InvalidFormat => 415,
            ApiError::ParseFailed => 422,
            ApiError::NotFound => 404,
            ApiError::Forbidden => 403,
            ApiError::AuthFailed => 403,
            ApiError::Duplicate => 409,
            ApiError::Internal => 500,
        }
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::FileTooLarge { max_bytes } => {
                write!(f, "File too large (max {}MB)", max_bytes / (1024 * 1024))
            }
            ApiError::AudioTooLong { max_duration_ms } => {
                write!(
                    f,
                    "Audio too long (max {} minutes)",
                    max_duration_ms / 60_000
                )
            }
            ApiError::InvalidFormat => write!(f, "Invalid audio format (must be Ogg/Opus)"),
            ApiError::ParseFailed => write!(f, "Failed to parse audio file"),
            ApiError::NotFound => write!(f, "Not found"),
            ApiError::Forbidden => write!(f, "Not authorized"),
            ApiError::AuthFailed => write!(f, "Authentication failed"),
            ApiError::Duplicate => write!(f, "Duplicate request"),
            ApiError::Internal => write!(f, "Internal server error"),
        }
    }
}

impl std::error::Error for ApiError {}
