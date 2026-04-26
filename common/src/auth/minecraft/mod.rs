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
    AccessTokenResponse, MinecraftJavaProfile, MinecraftLoginResponse, ProfileResponse,
    XboxAuthResponse,
};

#[derive(Debug, Clone)]
pub struct MinecraftAuthEndpoints {
    pub token_url: String,
    pub xbl_user_auth_url: String,
    pub xsts_authorize_url: String,
    pub user_presence_url: String,
    pub profile_settings_url: String,
    pub mc_login_with_xbox_url: String,
    pub mc_profile_url: String,
}

impl Default for MinecraftAuthEndpoints {
    fn default() -> Self {
        Self {
            token_url: "https://login.live.com/oauth20_token.srf".into(),
            xbl_user_auth_url: "https://user.auth.xboxlive.com/user/authenticate".into(),
            xsts_authorize_url: "https://xsts.auth.xboxlive.com/xsts/authorize".into(),
            user_presence_url: "https://userpresence.xboxlive.com/users/me".into(),
            profile_settings_url: "https://profile.xboxlive.com/users/batch/profile/settings".into(),
            mc_login_with_xbox_url: "https://api.minecraftservices.com/authentication/login_with_xbox".into(),
            mc_profile_url: "https://api.minecraftservices.com/minecraft/profile".into(),
        }
    }
}

/// Minecraft authentication provider using Xbox Live
pub struct MinecraftAuthProvider {
    client_id: String,
    pub(crate) endpoints: MinecraftAuthEndpoints,
}

impl MinecraftAuthProvider {
    pub fn new(client_id: String) -> Self {
        Self { client_id, endpoints: MinecraftAuthEndpoints::default() }
    }

    #[cfg(test)]
    pub(crate) fn with_endpoints(client_id: String, endpoints: MinecraftAuthEndpoints) -> Self {
        Self { client_id, endpoints }
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

        let token = self
            .exchange_code_for_token(&client, &code, &redirect_uri)
            .await?;

        let (xbl_token, user_hash) = self.authenticate_xbox_live(&client, &token).await?;

        let xsts_token = self.get_xsts_token(&client, &xbl_token).await?;

        let xuid = self.get_user_xuid(&client, &user_hash, &xsts_token).await?;

        let xbl_profile_fut = self.get_user_profile(&client, &user_hash, &xsts_token, &xuid);
        let java_profile_fut = self.get_minecraft_java_profile(&client, &user_hash, &xbl_token);

        let (xbl_result, java_result) = tokio::join!(xbl_profile_fut, java_profile_fut);

        let profile = xbl_result?;

        let (minecraft_username, minecraft_uuid) = match java_result {
            Ok(p) => (Some(p.name), Some(p.uuid)),
            Err(e) => {
                tracing::info!("Java profile not available (account likely lacks Java license): {}", e);
                (None, None)
            }
        };

        Ok(profile.with_java_profile(minecraft_username, minecraft_uuid))
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
            .map(|p| p.name)
    }

    async fn exchange_code_for_token(
        &self,
        client: &reqwest::Client,
        code: &str,
        redirect_uri: &Url,
    ) -> Result<String, AuthError> {
        let response = client
            .post(&self.endpoints.token_url)
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
            .post(&self.endpoints.xbl_user_auth_url)
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
            .post(&self.endpoints.xsts_authorize_url)
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
            .get(&self.endpoints.user_presence_url)
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
            .post(&self.endpoints.profile_settings_url)
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

    async fn get_minecraft_java_profile(
        &self,
        client: &reqwest::Client,
        user_hash: &str,
        xbl_token: &str,
    ) -> Result<MinecraftJavaProfile, AuthError> {
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
            .post(&self.endpoints.mc_login_with_xbox_url)
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
            .get(&self.endpoints.mc_profile_url)
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

        let profile: MinecraftJavaProfile = profile_response
            .json()
            .await
            .map_err(|e| AuthError::InvalidResponse(e.to_string()))?;

        Ok(profile)
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
            .post(&self.endpoints.xsts_authorize_url)
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

#[async_trait::async_trait]
impl crate::auth::MinecraftAuthenticator for MinecraftAuthProvider {
    async fn authenticate(
        &self,
        code: String,
        redirect_uri: reqwest::Url,
    ) -> Result<crate::auth::AuthResult, crate::auth::AuthError> {
        MinecraftAuthProvider::authenticate(self, code, redirect_uri).await
    }

    async fn authenticate_for_java_profile(
        &self,
        code: String,
        redirect_uri: reqwest::Url,
    ) -> Result<String, crate::auth::AuthError> {
        MinecraftAuthProvider::authenticate_for_java_profile(self, code, redirect_uri).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_default_uses_production_urls() {
        let e = MinecraftAuthEndpoints::default();
        assert_eq!(e.token_url, "https://login.live.com/oauth20_token.srf");
        assert_eq!(e.xbl_user_auth_url, "https://user.auth.xboxlive.com/user/authenticate");
        assert_eq!(e.xsts_authorize_url, "https://xsts.auth.xboxlive.com/xsts/authorize");
        assert_eq!(e.user_presence_url, "https://userpresence.xboxlive.com/users/me");
        assert_eq!(e.profile_settings_url, "https://profile.xboxlive.com/users/batch/profile/settings");
        assert_eq!(e.mc_login_with_xbox_url, "https://api.minecraftservices.com/authentication/login_with_xbox");
        assert_eq!(e.mc_profile_url, "https://api.minecraftservices.com/minecraft/profile");
    }

    #[test]
    fn provider_default_constructor_uses_production_endpoints() {
        let p = MinecraftAuthProvider::new("client-id".into());
        assert!(p.endpoints.token_url.starts_with("https://login.live.com"));
    }

    async fn mount_xbl_mocks(server: &wiremock::MockServer) {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, ResponseTemplate};

        Mock::given(method("POST")).and(path("/oauth20_token.srf"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "ms-token",
                "expires_in": 3600,
                "refresh_token": "rt",
                "token_type": "bearer"
            })))
            .mount(server).await;

        Mock::given(method("POST")).and(path("/user/authenticate"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "Token": "xbl-token",
                "DisplayClaims": { "xui": [{ "uhs": "user-hash" }] }
            })))
            .mount(server).await;

        Mock::given(method("POST")).and(path("/xsts/authorize"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "Token": "xsts-token",
                "DisplayClaims": { "xui": [{ "uhs": "user-hash" }] }
            })))
            .mount(server).await;

        Mock::given(method("GET")).and(path("/users/me"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "xuid": "1234567890"
            })))
            .mount(server).await;

        Mock::given(method("POST")).and(path("/users/batch/profile/settings"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "profileUsers": [{
                    "id": "1234567890",
                    "settings": [
                        { "id": "Gamertag", "value": "AwesomeXboxer123" },
                        { "id": "GameDisplayPicRaw", "value": "" }
                    ]
                }]
            })))
            .mount(server).await;
    }

    #[tokio::test]
    async fn authenticate_happy_path_returns_gamertag_and_gamerpic() {
        use wiremock::MockServer;

        let server = MockServer::start().await;
        mount_xbl_mocks(&server).await;

        let endpoints = MinecraftAuthEndpoints {
            token_url: format!("{}/oauth20_token.srf", server.uri()),
            xbl_user_auth_url: format!("{}/user/authenticate", server.uri()),
            xsts_authorize_url: format!("{}/xsts/authorize", server.uri()),
            user_presence_url: format!("{}/users/me", server.uri()),
            profile_settings_url: format!("{}/users/batch/profile/settings", server.uri()),
            mc_login_with_xbox_url: format!("{}/authentication/login_with_xbox", server.uri()),
            mc_profile_url: format!("{}/minecraft/profile", server.uri()),
        };

        let provider = MinecraftAuthProvider::with_endpoints("cid".into(), endpoints);

        let result = provider
            .authenticate("auth-code".into(), "http://app/cb".parse().unwrap())
            .await
            .expect("authenticate ok");

        assert_eq!(result.gamertag, "AwesomeXboxer123");
        assert!(result.minecraft_username.is_none());
        assert!(result.minecraft_uuid.is_none());
    }

    #[tokio::test]
    async fn authenticate_returns_java_profile_when_present() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        mount_xbl_mocks(&server).await;

        Mock::given(method("POST")).and(path("/authentication/login_with_xbox"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "mc-access"
            })))
            .mount(&server).await;

        Mock::given(method("GET")).and(path("/minecraft/profile"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "java-uuid-1", "name": "CoolBuilder42"
            })))
            .mount(&server).await;

        let endpoints = MinecraftAuthEndpoints {
            token_url: format!("{}/oauth20_token.srf", server.uri()),
            xbl_user_auth_url: format!("{}/user/authenticate", server.uri()),
            xsts_authorize_url: format!("{}/xsts/authorize", server.uri()),
            user_presence_url: format!("{}/users/me", server.uri()),
            profile_settings_url: format!("{}/users/batch/profile/settings", server.uri()),
            mc_login_with_xbox_url: format!("{}/authentication/login_with_xbox", server.uri()),
            mc_profile_url: format!("{}/minecraft/profile", server.uri()),
        };

        let provider = MinecraftAuthProvider::with_endpoints("cid".into(), endpoints);

        let result = provider
            .authenticate("auth-code".into(), "http://app/cb".parse().unwrap())
            .await
            .expect("authenticate ok");

        assert_eq!(result.gamertag, "AwesomeXboxer123");
        assert_eq!(result.minecraft_username.as_deref(), Some("CoolBuilder42"));
        assert_eq!(result.minecraft_uuid.as_deref(), Some("java-uuid-1"));
    }

    #[test]
    fn minecraft_auth_provider_is_object_safe() {
        fn assert_object_safe(_: &dyn crate::auth::MinecraftAuthenticator) {}
        let p = MinecraftAuthProvider::new("cid".into());
        assert_object_safe(&p);
    }

    #[tokio::test]
    async fn authenticate_handles_no_java_license() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        mount_xbl_mocks(&server).await;

        Mock::given(method("POST")).and(path("/authentication/login_with_xbox"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "mc-access"
            })))
            .mount(&server).await;

        Mock::given(method("GET")).and(path("/minecraft/profile"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server).await;

        let endpoints = MinecraftAuthEndpoints {
            token_url: format!("{}/oauth20_token.srf", server.uri()),
            xbl_user_auth_url: format!("{}/user/authenticate", server.uri()),
            xsts_authorize_url: format!("{}/xsts/authorize", server.uri()),
            user_presence_url: format!("{}/users/me", server.uri()),
            profile_settings_url: format!("{}/users/batch/profile/settings", server.uri()),
            mc_login_with_xbox_url: format!("{}/authentication/login_with_xbox", server.uri()),
            mc_profile_url: format!("{}/minecraft/profile", server.uri()),
        };

        let provider = MinecraftAuthProvider::with_endpoints("cid".into(), endpoints);

        let result = provider
            .authenticate("auth-code".into(), "http://app/cb".parse().unwrap())
            .await
            .expect("authenticate ok");

        assert_eq!(result.gamertag, "AwesomeXboxer123");
        assert!(result.minecraft_username.is_none());
        assert!(result.minecraft_uuid.is_none());
    }
}
