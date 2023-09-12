use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthCodeResponse {
    pub user_code: String,
    pub device_code: String,
    pub verification_uri: String,
    pub expires_in: i64,
    pub interval: u64,
    pub message: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthTokenResponse {
    pub token_type: String,
    pub scope: String,
    pub expires_in: i64,
    pub ext_expires_in: i64,
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct XboxLiveAuthResponse {
    pub issue_instant: String,
    pub not_after: String,
    pub token: String,
    pub display_claims: HashMap<String, Vec<HashMap<String, String>>>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct MinecraftAuthResponse {
    pub username: String,
    pub roles: Vec<String>,
    pub access_token: String,
    pub expires_in: u32,
    pub token_type: String,
}

#[derive(Error, Debug)]
pub enum AuthServiceError {
    #[error("The access token is invalid or was expired.")]
    InvalidAccessToken,

    #[error("An unexpected error has ocurred.")]
    UnknownError,

    #[error("{0}")]
    Request(#[from] reqwest::Error),

    #[error("{0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct AuthServiceErrorMessage {
    error: String,
}

pub struct AuthFlow {
    auth_code_res: Option<AuthCodeResponse>,
    auth_token_res: Option<AuthTokenResponse>,
    xbox_auth_res: Option<XboxLiveAuthResponse>,
    minecraft_res: Option<MinecraftAuthResponse>,
    client_id: String,

    client: Client,
}

impl AuthFlow {
    pub fn new(client_id: &str) -> Self {
        Self {
            client: Client::new(),

            auth_code_res: None,
            auth_token_res: None,
            xbox_auth_res: None,
            minecraft_res: None,
            client_id: client_id.to_string(),
        }
    }

    pub async fn request_code(&mut self) -> Result<&AuthCodeResponse, AuthServiceError> {
        let client_id = &self.client_id;

        let response = self
            .client
            .get("https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode")
            .query(&[
                ("client_id", client_id),
                ("scope", &"XboxLive.signin offline_access".to_string()),
            ])
            .send()
            .await?
            .json::<AuthCodeResponse>()
            .await?;

        self.auth_code_res = Some(response);
        return Ok(self.auth_code_res.as_ref().unwrap());
    }

    pub async fn wait_for_login(&mut self) -> Result<&AuthTokenResponse, AuthServiceError> {
        let auth_code = self.auth_code_res.as_ref().unwrap();
        let client_id = &self.client_id;

        loop {
            std::thread::sleep(std::time::Duration::from_secs(auth_code.interval + 1));

            let response = self
                .client
                .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
                .form(&[
                    ("client_id", client_id),
                    ("scope", &"XboxLive.signin offline_access".to_string()),
                    (
                        "grant_type",
                        &"urn:ietf:params:oauth:grant-type:device_code".to_string(),
                    ),
                    ("device_code", &auth_code.device_code),
                ])
                .send()
                .await?;

            match response.status() {
                StatusCode::BAD_REQUEST => {
                    let error: AuthServiceErrorMessage = response.json::<AuthServiceErrorMessage>().await?;
                    match &error.error as &str {
                        "authorization_declined" => {
                            return Err(AuthServiceError::InvalidAccessToken);
                        }
                        "expired_token" => {
                            return Err(AuthServiceError::InvalidAccessToken);
                        }
                        "invalid_grant" => {
                            return Err(AuthServiceError::InvalidAccessToken);
                        }
                        _ => {
                            continue;
                        }
                    }
                }

                StatusCode::OK => {
                    let result: AuthTokenResponse = response.json::<AuthTokenResponse>().await?;
                    self.auth_token_res = Some(result);
                    return Ok(self.auth_token_res.as_ref().unwrap());
                }
                _ => {
                    return Err(AuthServiceError::UnknownError);
                }
            }
        }
    }

    pub async fn login_in_xbox_live(&mut self) -> Result<&XboxLiveAuthResponse, AuthServiceError> {
        let auth_token = self.auth_token_res.as_ref().unwrap();

        let xbox_authenticate_json = json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                "RpsTicket": &format!("d={}", auth_token.access_token)
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT"
        });

        let xbox_res = self
            .client
            .post("https://user.auth.xboxlive.com/user/authenticate")
            .json(&xbox_authenticate_json)
            .send()
            .await?
            .json::<XboxLiveAuthResponse>()
            .await;

        self.xbox_auth_res = Some(xbox_res.unwrap());
        return Ok(self.xbox_auth_res.as_ref().unwrap());
    }

    pub async fn login_in_minecraft(&mut self) -> Result<serde_json::Value, AuthServiceError> {
        let xbox_res = self.xbox_auth_res.as_ref().unwrap();
        let xbox_token = &xbox_res.token;
        let user_hash = &xbox_res.display_claims["xui"][0]["uhs"];

        let xbox_security_token_res = self
            .client
            .post("https://xsts.auth.xboxlive.com/xsts/authorize")
            .json(&json!({
                "Properties": {
                    "SandboxId": "RETAIL",
                    "UserTokens": [xbox_token]
                },
                "RelyingParty": "rp://api.minecraftservices.com/",
                "TokenType": "JWT"
            }))
            .send()
            .await?
            .json::<XboxLiveAuthResponse>()
            .await?;

        let xbox_security_token = &xbox_security_token_res.token;

        dbg!("{}", & xbox_security_token);
        let minecraft_resp = self
            .client
            .post("https://api.minecraftservices.com/authentication/login_with_xbox")
            .json(&json!({
                "ensureLegacyEnabled": true,
                "identityToken":
                    format!(
                        "XBL3.0 x={user_hash};{xsts_token}",
                        user_hash = user_hash,
                        xsts_token = xbox_security_token
                    )
            }))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        dbg!("{:?", minecraft_resp.clone());
        return Ok(minecraft_resp.clone());
        //self.minecraft_res = Some(minecraft_resp);
        //dbg!(&self.minecraft_res);
        //return Ok(self.minecraft_res.as_ref().unwrap());
    }
}
