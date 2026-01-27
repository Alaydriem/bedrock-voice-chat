use thiserror::Error;

/// Errors that can occur during authentication
#[derive(Debug, Error)]
pub enum AuthError {
    /// Network/HTTP request failed
    #[error("Network error: {0}")]
    Network(String),

    /// Authentication was rejected by the provider
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    /// User profile was not found after successful auth
    #[error("Profile not found")]
    ProfileNotFound,

    /// Response from provider was malformed or unexpected
    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    /// Underlying HTTP client error
    #[error("Request error: {0}")]
    Request(#[from] reqwest::Error),
}