//! Minecraft authentication via Xbox Live OAuth
//!
//! This module provides authentication for Minecraft players using Microsoft/Xbox Live.
//! The authentication flow is:
//! 1. Client obtains OAuth code from Microsoft login
//! 2. Server exchanges code for Xbox Live tokens
//! 3. Server fetches player profile (gamertag, gamerpic)

mod dtos;

use base64::{engine::general_purpose, Engine as _};
use reqwest::header::HeaderMap;
use reqwest::Url;

use crate::auth::provider::{AuthError, AuthResult};
use dtos::{
    AccessTokenResponse, MinecraftLoginResponse, MinecraftProfileResponse, ProfileResponse,
    XboxAuthResponse,
};

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

        let minecraft_username: Option<String> = None;

        Ok(profile.with_minecraft_username(minecraft_username))
    }
}

impl MinecraftAuthProvider {
    /// Authenticate only enough to get the Minecraft Java profile.
    /// Runs Steps 1 (token exchange), 2 (XBL auth), and 6 (MC Services).
    /// Skips Xbox profile/XUID lookup since we only need the MC username.
    pub async fn authenticate_for_java_profile(
        &self,
        code: String,
        redirect_uri: Url,
    ) -> Result<String, AuthError> {
        let client = reqwest::Client::builder()
            .build()
            .map_err(|e| AuthError::Network(e.to_string()))?;

        let token = self
            .exchange_code_for_token(&client, &code, &redirect_uri)
            .await?;

        let (xbl_token, user_hash) = self.authenticate_xbox_live(&client, &token).await?;

        self.get_minecraft_java_profile(&client, &user_hash, &xbl_token)
            .await
    }

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

    /// Get the player's Minecraft Java Edition profile (username + UUID).
    /// Returns None if the player doesn't own Java Edition.
    async fn get_minecraft_java_profile(
        &self,
        client: &reqwest::Client,
        user_hash: &str,
        xbl_token: &str,
    ) -> Result<String, AuthError> {
        // Get a separate XSTS token for Minecraft Services
        let mc_xsts_token = self
            .get_minecraft_xsts_token(client, xbl_token)
            .await
            .map_err(|e| {
                tracing::warn!("MC Services: XSTS token request failed: {}", e);
                e
            })?;

        // Authenticate with Minecraft Services
        let identity_token = format!("XBL3.0 x={};{}", user_hash, mc_xsts_token);

        let mc_login_response = client
            .post("https://api.minecraftservices.com/authentication/login_with_xbox")
            .json(&serde_json::json!({
                "identityToken": identity_token,
            }))
            .send()
            .await?;

        let mc_login_status = mc_login_response.status();
        if !mc_login_status.is_success() {
            let body = mc_login_response.text().await.unwrap_or_default();
            tracing::error!("MC Services: login_with_xbox failed ({}): {}", mc_login_status, body);
            return Err(AuthError::AuthenticationFailed(format!(
                "MC Services login failed ({}): {}",
                mc_login_status, body
            )));
        }

        let mc_login: MinecraftLoginResponse = mc_login_response
            .json()
            .await
            .map_err(|e| AuthError::InvalidResponse(e.to_string()))?;
        tracing::info!("MC Services: Got MC access token successfully");

        // Fetch the Minecraft profile
        tracing::info!("MC Services: Fetching Minecraft profile");
        let profile_response = client
            .get("https://api.minecraftservices.com/minecraft/profile")
            .header("Authorization", format!("Bearer {}", mc_login.access_token))
            .send()
            .await?;

        let profile_status = profile_response.status();
        if !profile_status.is_success() {
            let body = profile_response.text().await.unwrap_or_default();
            tracing::warn!("MC Services: profile fetch failed ({}): {}", profile_status, body);
            return Err(AuthError::AuthenticationFailed(format!(
                "MC profile fetch failed ({})",
                profile_status
            )));
        }

        let profile: MinecraftProfileResponse = profile_response
            .json()
            .await
            .map_err(|e| AuthError::InvalidResponse(e.to_string()))?;

        Ok(profile.name)
    }

    /// Get an XSTS token authorized for Minecraft Services
    async fn get_minecraft_xsts_token(
        &self,
        client: &reqwest::Client,
        xbl_token: &str,
    ) -> Result<String, AuthError> {
        let json = serde_json::json!({
            "Properties": {
                "SandboxId": "RETAIL",
                "UserTokens": [xbl_token]
            },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT"
        });

        let mut headers = HeaderMap::new();
        headers.insert("Accept", "application/json".parse().unwrap());
        headers.insert("Content-Type", "application/json".parse().unwrap());

        let xsts_response = client
            .post("https://xsts.auth.xboxlive.com/xsts/authorize")
            .json(&json)
            .headers(headers)
            .send()
            .await?;

        let xsts_status = xsts_response.status();
        if !xsts_status.is_success() {
            let body = xsts_response.text().await.unwrap_or_default();
            tracing::warn!("MC Services: XSTS authorize failed ({}): {}", xsts_status, body);
            return Err(AuthError::AuthenticationFailed(format!(
                "MC XSTS authorize failed ({})",
                xsts_status
            )));
        }

        let response: XboxAuthResponse = xsts_response
            .json()
            .await
            .map_err(|e| AuthError::InvalidResponse(e.to_string()))?;

        Ok(response.token)
    }
}
