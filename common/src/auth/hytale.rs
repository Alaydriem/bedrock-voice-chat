//! Hytale authentication via OAuth2 Device Code Flow (RFC 8628)
//!
//! This module provides authentication for Hytale players using the device code flow.
//! The authentication flow is:
//! 1. Server starts device flow, receives user code
//! 2. User visits verification URL and enters code
//! 3. Server polls for completion
//! 4. On success, server fetches player profile

use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};

use super::provider::{AuthError, AuthResult};

// ============================================================================
// Constants
// ============================================================================

const DEVICE_AUTH_URL: &str = "https://oauth.accounts.hytale.com/oauth2/device/auth";
const TOKEN_URL: &str = "https://oauth.accounts.hytale.com/oauth2/token";
const PROFILE_URL: &str = "https://account-data.hytale.com/my-account/get-profiles";

const CLIENT_ID: &str = "hytale-server";
const SCOPE: &str = "openid offline auth:server";
const GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:device_code";

// ============================================================================
// Public API
// ============================================================================

/// Hytale authentication provider using OAuth2 Device Code Flow
#[derive(Debug, Clone, Default)]
pub struct HytaleAuthProvider;

impl HytaleAuthProvider {
    /// Create a new Hytale authentication provider
    pub fn new() -> Self {
        Self
    }

    /// Start a device code flow
    ///
    /// Returns a `DeviceFlow` containing the user code and verification URL
    /// to display to the user.
    pub async fn start_device_flow(&self) -> Result<DeviceFlow, AuthError> {
        let client = reqwest::Client::builder()
            .build()
            .map_err(|e| AuthError::Network(e.to_string()))?;

        let response = client
            .post(DEVICE_AUTH_URL)
            .form(&[("client_id", CLIENT_ID), ("scope", SCOPE)])
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AuthError::AuthenticationFailed(format!(
                "Device authorization failed ({}): {}",
                status, body
            )));
        }

        let device_response: DeviceCodeResponse = response
            .json()
            .await
            .map_err(|e| AuthError::InvalidResponse(e.to_string()))?;

        // Build verification_uri_complete if not provided
        let verification_uri_complete = device_response
            .verification_uri_complete
            .unwrap_or_else(|| {
                format!(
                    "{}?user_code={}",
                    device_response.verification_uri, device_response.user_code
                )
            });

        Ok(DeviceFlow {
            device_code: device_response.device_code,
            user_code: device_response.user_code,
            verification_uri: device_response.verification_uri,
            verification_uri_complete,
            expires_in: device_response.expires_in,
            interval: device_response.interval,
        })
    }

    /// Poll for device flow completion
    ///
    /// Call this periodically (respecting the interval from `DeviceFlow`)
    /// until it returns `PollResult::Success` or a terminal state.
    pub async fn poll(&self, flow: &DeviceFlow) -> Result<PollResult, AuthError> {
        let client = reqwest::Client::builder()
            .build()
            .map_err(|e| AuthError::Network(e.to_string()))?;

        let response = client
            .post(TOKEN_URL)
            .form(&[
                ("grant_type", GRANT_TYPE),
                ("device_code", &flow.device_code),
                ("client_id", CLIENT_ID),
            ])
            .send()
            .await?;

        let status = response.status();

        if status.is_success() {
            let token_response: TokenResponse = response
                .json()
                .await
                .map_err(|e| AuthError::InvalidResponse(e.to_string()))?;

            // Fetch user profile with the access token
            let profile = self.get_profile(&token_response.access_token).await?;

            return Ok(PollResult::Success(profile));
        }

        // Parse error response
        let error_body = response.text().await.unwrap_or_default();

        if let Ok(error_response) = serde_json::from_str::<TokenErrorResponse>(&error_body) {
            return Ok(match error_response.error.as_str() {
                "authorization_pending" => PollResult::Pending,
                "slow_down" => PollResult::SlowDown,
                "expired_token" => PollResult::Expired,
                "access_denied" => PollResult::Denied,
                _ => {
                    return Err(AuthError::AuthenticationFailed(format!(
                        "{}: {}",
                        error_response.error,
                        error_response.error_description.unwrap_or_default()
                    )));
                }
            });
        }

        Err(AuthError::InvalidResponse(format!(
            "Token request failed ({}): {}",
            status, error_body
        )))
    }
}

// ============================================================================
// Private Implementation
// ============================================================================

impl HytaleAuthProvider {
    async fn get_profile(&self, access_token: &str) -> Result<AuthResult, AuthError> {
        let client = reqwest::Client::builder()
            .build()
            .map_err(|e| AuthError::Network(e.to_string()))?;

        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            format!("Bearer {}", access_token)
                .parse()
                .map_err(|_| AuthError::InvalidResponse("Invalid access token".to_string()))?,
        );

        let response = client.get(PROFILE_URL).headers(headers).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AuthError::AuthenticationFailed(format!(
                "Profile request failed ({}): {}",
                status, body
            )));
        }

        let profile_response: HytaleProfileResponse = response
            .json()
            .await
            .map_err(|e| AuthError::InvalidResponse(e.to_string()))?;

        let profile = profile_response
            .profiles
            .into_iter()
            .next()
            .ok_or(AuthError::ProfileNotFound)?;

        // Hytale doesn't provide a gamerpic URL in the same way Xbox Live does
        Ok(AuthResult::without_gamerpic(profile.username))
    }
}

// ============================================================================
// Public Types
// ============================================================================

/// An active device code flow session
///
/// Contains the information needed to:
/// - Display to the user (user_code, verification URLs)
/// - Poll for completion (device_code - crate visibility)
/// - Manage timing (expires_in, interval)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceFlow {
    /// The device code (used internally for polling)
    #[serde(skip_serializing)]
    pub(crate) device_code: String,

    /// The code to display to the user
    pub user_code: String,

    /// The URL where the user enters the code
    pub verification_uri: String,

    /// The URL with the code pre-filled (convenience for users)
    pub verification_uri_complete: String,

    /// How long until the device code expires (seconds)
    pub expires_in: u64,

    /// Minimum interval between poll requests (seconds)
    pub interval: u64,
}

impl DeviceFlow {
    /// Get the device code for session storage
    pub fn device_code(&self) -> &str {
        &self.device_code
    }
}

/// Result of polling for device flow completion
#[derive(Debug, Clone)]
pub enum PollResult {
    /// Authorization is pending - user hasn't completed yet
    Pending,

    /// Polling too fast - slow down
    SlowDown,

    /// Successfully authenticated
    Success(AuthResult),

    /// Device code has expired
    Expired,

    /// User denied the authorization
    Denied,
}

// ============================================================================
// Private Types (API response structures)
// ============================================================================

#[derive(Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    #[serde(default)]
    verification_uri_complete: Option<String>,
    expires_in: u64,
    interval: u64,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct TokenErrorResponse {
    error: String,
    #[serde(default)]
    error_description: Option<String>,
}

#[derive(Deserialize)]
struct HytaleProfileResponse {
    profiles: Vec<HytaleProfile>,
}

#[derive(Deserialize)]
struct HytaleProfile {
    username: String,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_flow_device_code_visibility() {
        let flow = DeviceFlow {
            device_code: "secret".to_string(),
            user_code: "ABCD-1234".to_string(),
            verification_uri: "https://example.com/device".to_string(),
            verification_uri_complete: "https://example.com/device?code=ABCD-1234".to_string(),
            expires_in: 600,
            interval: 5,
        };

        // device_code is accessible via method
        assert_eq!(flow.device_code(), "secret");

        // user_code is public
        assert_eq!(flow.user_code, "ABCD-1234");
    }

    #[test]
    fn test_device_flow_serialization_hides_device_code() {
        let flow = DeviceFlow {
            device_code: "secret".to_string(),
            user_code: "ABCD-1234".to_string(),
            verification_uri: "https://example.com/device".to_string(),
            verification_uri_complete: "https://example.com/device?code=ABCD-1234".to_string(),
            expires_in: 600,
            interval: 5,
        };

        let json = serde_json::to_string(&flow).unwrap();

        // device_code should not appear in serialized output
        assert!(!json.contains("secret"));
        assert!(json.contains("ABCD-1234"));
    }
}
