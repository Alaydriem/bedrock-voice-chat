//! Minecraft authentication via Xbox Live OAuth
//!
//! This module provides authentication for Minecraft players using Microsoft/Xbox Live.
//! The authentication flow is:
//! 1. Client obtains OAuth code from Microsoft login
//! 2. Server exchanges code for Xbox Live tokens
//! 3. Server fetches player profile (gamertag, gamerpic)

use base64::{engine::general_purpose, Engine as _};
use reqwest::header::HeaderMap;
use reqwest::Url;
use serde::Deserialize;

use super::provider::{AuthError, AuthResult};

// ============================================================================
// Public API
// ============================================================================

/// Minecraft authentication provider using Xbox Live
pub struct MinecraftAuthProvider {
    client_id: String,
}

impl MinecraftAuthProvider {
    /// Create a new provider with the given Microsoft OAuth client ID
    pub fn new(client_id: String) -> Self {
        Self { client_id }
    }

    /// Authenticate a user with an OAuth authorization code
    ///
    /// # Arguments
    /// * `code` - The OAuth authorization code from Microsoft login
    /// * `redirect_uri` - The redirect URI used in the OAuth flow
    ///
    /// # Returns
    /// * `Ok(AuthResult)` - The user's gamertag and gamerpic
    /// * `Err(AuthError)` - If authentication fails
    pub async fn authenticate(
        &self,
        code: String,
        redirect_uri: Url,
    ) -> Result<AuthResult, AuthError> {
        let client = reqwest::Client::builder()
            .build()
            .map_err(|e| AuthError::Network(e.to_string()))?;

        // Step 1: Exchange code for Microsoft access token
        let token = self
            .exchange_code_for_token(&client, &code, &redirect_uri)
            .await?;

        // Step 2: Authenticate with Xbox Live
        let (xbl_token, user_hash) = self.authenticate_xbox_live(&client, &token).await?;

        // Step 3: Get XSTS token
        let xsts_token = self.get_xsts_token(&client, &xbl_token).await?;

        // Step 4: Get user's XUID
        let xuid = self
            .get_user_xuid(&client, &user_hash, &xsts_token)
            .await?;

        // Step 5: Get user profile (gamertag and gamerpic)
        let profile = self
            .get_user_profile(&client, &user_hash, &xsts_token, &xuid)
            .await?;

        Ok(profile)
    }
}

// ============================================================================
// Private Implementation
// ============================================================================

impl MinecraftAuthProvider {
    async fn exchange_code_for_token(
        &self,
        client: &reqwest::Client,
        code: &str,
        redirect_uri: &Url,
    ) -> Result<String, AuthError> {
        let response = client
            .post("https://login.live.com/oauth20_token.srf")
            .form(&[
                ("client_id", self.client_id.clone()),
                ("code", code.to_string()),
                ("grant_type", "authorization_code".to_string()),
                ("redirect_uri", redirect_uri.to_string()),
            ])
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!("Token exchange failed ({}): {}", status, body);
            return Err(AuthError::AuthenticationFailed(format!(
                "Token exchange failed ({})",
                status
            )));
        }

        let token: AccessTokenResponse = response
            .json()
            .await
            .map_err(|e| AuthError::InvalidResponse(e.to_string()))?;

        Ok(token.access_token)
    }

    async fn authenticate_xbox_live(
        &self,
        client: &reqwest::Client,
        access_token: &str,
    ) -> Result<(String, String), AuthError> {
        let json = serde_json::json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                "RpsTicket": format!("d={}", access_token),
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT"
        });

        let response: XboxAuthResponse = client
            .post("https://user.auth.xboxlive.com/user/authenticate")
            .json(&json)
            .send()
            .await?
            .json()
            .await
            .map_err(|e| AuthError::InvalidResponse(e.to_string()))?;

        let user_hash = response
            .display_claims
            .xui
            .into_iter()
            .next()
            .map(|x| x.user_hash)
            .ok_or_else(|| AuthError::InvalidResponse("No XUI found in response".to_string()))?;

        Ok((response.token, user_hash))
    }

    async fn get_xsts_token(
        &self,
        client: &reqwest::Client,
        xbl_token: &str,
    ) -> Result<String, AuthError> {
        let json = serde_json::json!({
            "Properties": {
                "SandboxId": "RETAIL",
                "UserTokens": [xbl_token]
            },
            "RelyingParty": "http://xboxlive.com",
            "TokenType": "JWT"
        });

        let mut headers = HeaderMap::new();
        headers.insert("Accept", "application/json".parse().unwrap());
        headers.insert("Content-Type", "application/json".parse().unwrap());
        headers.insert("x-xbl-contract-version", "1".parse().unwrap());

        let response: XboxAuthResponse = client
            .post("https://xsts.auth.xboxlive.com/xsts/authorize")
            .json(&json)
            .headers(headers)
            .send()
            .await?
            .json()
            .await
            .map_err(|e| AuthError::InvalidResponse(e.to_string()))?;

        Ok(response.token)
    }

    async fn get_user_xuid(
        &self,
        client: &reqwest::Client,
        user_hash: &str,
        xsts_token: &str,
    ) -> Result<String, AuthError> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            format!("XBL3.0 x={};{}", user_hash, xsts_token)
                .parse()
                .unwrap(),
        );
        headers.insert("Accept", "application/json".parse().unwrap());
        headers.insert("Accept-Language", "en-US".parse().unwrap());
        headers.insert("x-xbl-contract-version", "3".parse().unwrap());
        headers.insert("Host", "userpresence.xboxlive.com".parse().unwrap());

        let presence: serde_json::Value = client
            .get("https://userpresence.xboxlive.com/users/me")
            .headers(headers)
            .send()
            .await?
            .json()
            .await
            .map_err(|e| AuthError::InvalidResponse(e.to_string()))?;

        presence["xuid"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| AuthError::InvalidResponse("No XUID in presence response".to_string()))
    }

    async fn get_user_profile(
        &self,
        client: &reqwest::Client,
        user_hash: &str,
        xsts_token: &str,
        xuid: &str,
    ) -> Result<AuthResult, AuthError> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            format!("XBL3.0 x={};{}", user_hash, xsts_token)
                .parse()
                .unwrap(),
        );
        headers.insert("x-xbl-contract-version", "3".parse().unwrap());

        let profile: ProfileResponse = client
            .post("https://profile.xboxlive.com/users/batch/profile/settings")
            .json(&serde_json::json!({
                "userIds": vec![xuid],
                "settings": vec![
                    "GameDisplayPicRaw",
                    "Gamertag"
                ]
            }))
            .headers(headers)
            .send()
            .await?
            .json()
            .await
            .map_err(|e| AuthError::InvalidResponse(e.to_string()))?;

        // Extract gamertag and gamerpic from profile
        let user = profile
            .profile_users
            .into_iter()
            .next()
            .ok_or(AuthError::ProfileNotFound)?;

        let mut gamertag: Option<String> = None;
        let mut gamerpic: Option<String> = None;

        for setting in user.settings {
            match setting.id.as_str() {
                "Gamertag" => gamertag = Some(setting.value),
                "GameDisplayPicRaw" => {
                    gamerpic = Some(general_purpose::STANDARD.encode(&setting.value))
                }
                _ => {}
            }
        }

        match (gamertag, gamerpic) {
            (Some(tag), Some(pic)) => Ok(AuthResult::new(tag, pic)),
            (Some(tag), None) => Ok(AuthResult::without_gamerpic(tag)),
            _ => Err(AuthError::InvalidResponse(
                "Profile missing required attributes".to_string(),
            )),
        }
    }
}

// ============================================================================
// Private Types (API response structures)
// ============================================================================

#[derive(Deserialize)]
struct AccessTokenResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct XboxAuthResponse {
    #[serde(rename = "Token")]
    token: String,
    #[serde(rename = "DisplayClaims")]
    display_claims: DisplayClaims,
}

#[derive(Deserialize)]
struct DisplayClaims {
    xui: Vec<Xui>,
}

#[derive(Deserialize)]
struct Xui {
    #[serde(rename = "uhs")]
    user_hash: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProfileResponse {
    profile_users: Vec<ProfileUser>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProfileUser {
    settings: Vec<Setting>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Setting {
    id: String,
    value: String,
}
