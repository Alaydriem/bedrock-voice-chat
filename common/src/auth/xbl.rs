use anyhow::Context;
use rand::distributions::Alphanumeric;
use rand::Rng;
use reqwest::header::HeaderMap;
use reqwest::{get, Url};
use rocket::{get, routes, Shutdown};
use serde::Deserialize;
use std::borrow::Cow;
use std::env;

#[derive(Deserialize, Clone)]
pub struct Query {
    pub code: String,
    pub state: String,
}

#[derive(Deserialize)]
pub struct AccessToken {
    pub access_token: String,
}

#[derive(Deserialize)]
pub struct Xui {
    #[serde(rename = "uhs")]
    pub user_hash: String,
}

#[derive(Deserialize)]
pub struct DisplayClaims {
    pub xui: Vec<Xui>,
}

#[derive(Deserialize)]
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

#[doc(hidden)]
static mut RESPONSE_DATA: Option<Query> = None;

#[doc(hidden)]
#[get("/?<code>&<state>")]
fn index(shutdown: Shutdown, code: &str, state: &str) {
    unsafe {
        RESPONSE_DATA = Some(Query {
            code: code.to_string(),
            state: state.to_string(),
        })
    };
    shutdown.notify();
}

/// Start listening on a random port for the callback
async fn receive_query(port: u16) -> Option<Query> {
    let figment = rocket::Config::figment()
        .merge(("address", "127.0.0.1"))
        .merge(("port", port));

    let _ = rocket::custom(figment)
        .mount("/", routes![index])
        .ignite()
        .await
        .unwrap()
        .launch()
        .await;

    // Extract the response
    let data: Option<Query> = unsafe { RESPONSE_DATA.clone() };
    data
}

/// Generates a random string for the state token
fn random_string() -> String {
    rand::thread_rng()
        .sample_iter(Alphanumeric)
        .take(32)
        .map(char::from)
        .collect()
}

/// The client should call client_authenticate_step_1 with the embeded client_id provided by the server
/// This will return a URL that should be presented to the client to be opened in a web browser
pub async fn client_authenticate_step_1(
    client_id: String,
) -> anyhow::Result<(String, String), anyhow::Error> {
    let redirect_uri: Url = "http://localhost:8085"
        .parse()
        .context("redirect uri is not a valid url")?;

    let state = random_string();
    let url = format!(
        "https://login.live.com/oauth20_authorize.srf\
        ?client_id={}\
        &response_type=code\
        &redirect_uri={}\
        &scope=XboxLive.signin%20offline_access\
        &state={}",
        client_id, redirect_uri, state
    );

    return Ok((url, state));
}

/// Before opening the browser, the client should also initialize step_2, which sets a listener on 8085
/// And awaits a callback from the server.
/// Upon return, the client should call POST /api/auth with the code in the body parameters
pub async fn client_authenticate_step_2(state: String) -> anyhow::Result<String, anyhow::Error> {
    match receive_query(8085).await {
        Some(query) => {
            anyhow::ensure!(
                state == query.state,
                "state mismatch: got state '{}' from query, but expected state was '{}'",
                state,
                query.state
            );

            return Ok(query.code);
        }
        None => Err(anyhow::anyhow!("Did not receive code back from API")),
    }
}

/// Takes the OAuth2 state code from the client, and completes the OAuth2 transaction
/// This is used on the server to get the player's identity and information and persist it in the state
pub async fn server_authenticate_with_client_code(
    client_id: String,
    client_secret: String,
    code: String,
) -> anyhow::Result<serde_json::Value, anyhow::Error> {
    let client = reqwest::Client::builder()
        .connection_verbose(true)
        .build()
        .unwrap();

    let access_token: AccessToken = client
        .post("https://login.live.com/oauth20_token.srf")
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("code", code),
            ("grant_type", "authorization_code".to_string()),
            ("redirect_uri", "127.0.0.1".to_string()), // This shouldn't really matter, it just needs to be set
        ])
        .send()
        .await?
        .json()
        .await?;

    let access_token = access_token.access_token;
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
    let profile = client
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
        .json::<serde_json::Value>()
        .await?;

    return Ok(profile);
}
