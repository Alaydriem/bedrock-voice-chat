use anyhow::Context;
use reqwest::header::HeaderMap;
use reqwest::Url;
use serde::Deserialize;
use serde::Serialize;
use std::borrow::Cow;

#[derive(Deserialize, Clone)]
pub struct Query {
    pub code: String,
    pub state: String,
}

#[derive(Deserialize, Debug)]
pub struct AccessToken {
    pub access_token: String,
}

#[derive(Deserialize, Debug)]
pub struct Xui {
    #[serde(rename = "uhs")]
    pub user_hash: String,
}

#[derive(Deserialize, Debug)]
pub struct DisplayClaims {
    pub xui: Vec<Xui>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileResponse {
    pub profile_users: Vec<ProfileUser>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileUser {
    pub id: String,
    pub host_id: String,
    pub settings: Vec<Setting>,
    pub is_sponsored_user: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Setting {
    pub id: String,
    pub value: String,
}

#[derive(Deserialize, Debug)]
pub struct AuthenticateWithXboxLiveOrXsts {
    #[serde(rename = "Token")]
    pub token: String,

    #[serde(rename = "DisplayClaims")]
    pub display_claims: DisplayClaims,
}

#[derive(Deserialize, PartialEq)]
pub struct Item {
    pub name: Cow<'static, str>,
    // pub signature: String, // todo: signature verification
}

impl Item {
    pub const PRODUCT_MINECRAFT: Self = Self {
        name: Cow::Borrowed("product_minecraft"),
    };
    pub const GAME_MINECRAFT: Self = Self {
        name: Cow::Borrowed("game_minecraft"),
    };
}

#[derive(Deserialize)]
pub struct Store {
    pub items: Vec<Item>,

    // pub signature: String, // todo: signature verification
    #[serde(rename = "keyId")]
    pub key_id: String,
}

impl AuthenticateWithXboxLiveOrXsts {
    pub fn extract_essential_information(self) -> anyhow::Result<(String, String)> {
        let token = self.token;
        let user_hash = self
            .display_claims
            .xui
            .into_iter()
            .next()
            .context("no xui found")?
            .user_hash;

        Ok((token, user_hash))
    }
}

#[derive(Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
}

/// Takes the OAuth2 state code from the client, and completes the OAuth2 transaction
/// This is used on the server to get the player's identity and information and persist it in the state
pub async fn server_authenticate_with_client_code(
    client_id: String,
    _client_secret: String,
    code: String,
    redirect_uri: Url,
) -> anyhow::Result<ProfileResponse, anyhow::Error> {
    let client = reqwest::Client::builder()
        .connection_verbose(true)
        .build()
        .unwrap();

    let token: AccessToken = client
        .post("https://login.live.com/oauth20_token.srf")
        .form(&[
            ("client_id", client_id),
            ("code", code),
            ("grant_type", "authorization_code".to_string()),
            ("redirect_uri", redirect_uri.to_string()), // This shouldn't really matter, it just needs to be set
        ])
        .send()
        .await?
        .json()
        .await?;

    let access_token = token.access_token;
    let json = serde_json::json!({
        "Properties": {
            "AuthMethod": "RPS",
            "SiteName": "user.auth.xboxlive.com",
            "RpsTicket": format!("d={}", access_token),
        },
        "RelyingParty": "http://auth.xboxlive.com",
        "TokenType": "JWT"
    });

    let auth_with_xbl: AuthenticateWithXboxLiveOrXsts = client
        .post("https://user.auth.xboxlive.com/user/authenticate")
        .json(&json)
        .send()
        .await?
        .json()
        .await?;
    let (token, user_hash) = auth_with_xbl.extract_essential_information()?;

    let json = serde_json::json!({
        "Properties": {
            "SandboxId": "RETAIL",
            "UserTokens": [token]
        },
        "RelyingParty": "http://xboxlive.com",
        "TokenType": "JWT"
    });

    let mut headers = HeaderMap::new();

    headers.insert("Accept", "application/json".parse().unwrap());

    headers.insert("Content-Type", "application/json".parse().unwrap());
    headers.insert("x-xbl-contract-version", "1".parse().unwrap());

    let auth_with_xsts: AuthenticateWithXboxLiveOrXsts = client
        .post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .json(&json)
        .headers(headers.clone())
        .send()
        .await?
        .json()
        .await?;

    let (token, _) = auth_with_xsts.extract_essential_information()?;

    headers = HeaderMap::new();
    headers.insert(
        "Authorization",
        format!("XBL3.0 x={};{}", user_hash, token).parse().unwrap(),
    );
    headers.insert("Accept", "application/json".parse().unwrap());
    headers.insert("Accept-Language", "en-US".parse().unwrap());
    headers.insert("x-xbl-contract-version", "3".parse().unwrap());
    headers.insert("Host", "userpresence.xboxlive.com".parse().unwrap());

    let presence = client
        .get("https://userpresence.xboxlive.com/users/me")
        .headers(headers)
        .send()
        .await?
        .json::<serde_json::Value>()
        .await
        .unwrap();

    let xuid = presence["xuid"].as_str().unwrap();

    headers = HeaderMap::new();
    headers.insert(
        "Authorization",
        format!("XBL3.0 x={};{}", user_hash, token).parse().unwrap(),
    );
    headers.insert("x-xbl-contract-version", "3".parse().unwrap());
    let profile: ProfileResponse = client
        .post("https://profile.xboxlive.com/users/batch/profile/settings")
        .json(&serde_json::json!({
            "userIds": vec![&xuid],
            "settings": vec![
                "GameDisplayPicRaw",
                "Gamertag"
            ]
        }))
        .headers(headers.clone())
        .send()
        .await?
        .json()
        .await?;

    return Ok(profile);
}
