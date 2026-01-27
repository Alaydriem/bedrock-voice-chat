//! Hytale authentication via OAuth2 Device Code Flow (RFC 8628)
//!
//! This module provides authentication for Hytale players using the device code flow.
//! The authentication flow is:
//! 1. Server starts device flow, receives user code
//! 2. User visits verification URL and enters code
//! 3. Server polls for completion
//! 4. On success, server fetches player profile

mod dtos;

use base64::{engine::general_purpose, Engine as _};
use reqwest::header::HeaderMap;

use crate::auth::provider::{AuthError, AuthResult};
use dtos::{DeviceCodeResponse, HytaleProfileResponse, TokenErrorResponse, TokenResponse};

// Re-export public types
pub use dtos::{DeviceFlow, PollResult};

// OAuth endpoints
const DEVICE_AUTH_URL: &str = "https://oauth.accounts.hytale.com/oauth2/device/auth";
const TOKEN_URL: &str = "https://oauth.accounts.hytale.com/oauth2/token";
const PROFILE_URL: &str = "https://account-data.hytale.com/my-account/get-profiles";

// OAuth client configuration
const CLIENT_ID: &str = "hytale-server";
const SCOPE: &str = "openid offline auth:server";
const GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:device_code";

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

        Ok(DeviceFlow::new(
            device_response.device_code,
            device_response.user_code,
            device_response.verification_uri,
            verification_uri_complete,
            device_response.expires_in,
            device_response.interval,
        ))
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
                ("device_code", flow.device_code()),
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

    /// Fetch user profile from Hytale API
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

        let avatar_url = format!("https://crafthead.net/hytale/avatar/{}", profile.uuid);
        let gamerpic = general_purpose::STANDARD.encode(&avatar_url);

        Ok(AuthResult::new(profile.username, gamerpic))
    }
}
