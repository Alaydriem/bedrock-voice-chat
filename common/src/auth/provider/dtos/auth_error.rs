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

    /// The authorization code was refused when exchanged for a token.
    ///
    /// Separate from `AuthenticationFailed` because it says nothing about the account:
    /// the code was expired, already redeemed, or issued against a different redirect.
    /// The caller can offer another sign-in, which is the opposite of what it should
    /// offer when an account is genuinely not allowed through.
    #[error("Authorization code rejected: {0}")]
    CodeRejected(String),

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
