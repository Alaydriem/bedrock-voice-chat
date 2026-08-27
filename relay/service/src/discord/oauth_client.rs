use serde_json::Value;

use crate::config::DiscordConfig;

use super::error::DiscordError;

// Learns which Discord account is at the browser, once, at enrollment.
//
// The authorization-code flow rather than implicit: implicit returns the token in the
// URL fragment, which a server-side callback never receives. This registry holds a
// client secret, so it can use the flow a browser-only client cannot.
//
// Distinct from `DiscordBotClient`, which reads any member's roles with the relay's
// own credential. This one answers a different question — who is asking — and its
// token is discarded the moment it has.
pub struct DiscordOAuthClient {
    http: reqwest::Client,
    client_id: String,
    client_secret: String,
    redirect_uri: String,
    api_base: String,
}

impl DiscordOAuthClient {
    // The bot token reads roles, so nothing here needs `guilds.members.read`. Asking
    // for more than the account id would collect data this registry has no use for.
    pub const SCOPE: &'static str = "identify";

    const AUTHORIZE: &'static str = "https://discord.com/oauth2/authorize";
    const DEFAULT_API_BASE: &'static str = "https://discord.com/api/v10";
    const TOKEN_PATH: &'static str = "/oauth2/token";
    const ME_PATH: &'static str = "/users/@me";

    pub fn new(config: &DiscordConfig, redirect_uri: String) -> Self {
        Self::new_with_base(config, redirect_uri, Self::DEFAULT_API_BASE)
    }

    pub fn new_with_base(config: &DiscordConfig, redirect_uri: String, api_base: &str) -> Self {
        Self {
            http: reqwest::Client::new(),
            client_id: config.client_id.clone(),
            client_secret: config.client_secret.clone(),
            redirect_uri,
            api_base: api_base.trim_end_matches('/').to_string(),
        }
    }

    pub fn authorize_url(client_id: &str, redirect_uri: &str, state: &str) -> String {
        let mut url = url::Url::parse(Self::AUTHORIZE).expect("a static authorize URL is valid");
        url.query_pairs_mut()
            .append_pair("client_id", client_id)
            .append_pair("response_type", "code")
            .append_pair("scope", Self::SCOPE)
            .append_pair("redirect_uri", redirect_uri)
            .append_pair("state", state)
            .append_pair("prompt", "none");
        url.to_string()
    }

    // The Discord user id, and nothing else kept. The access token is used once to
    // ask who this is and then dropped: a registry that stored it would be holding a
    // credential it has no further use for.
    pub async fn identify(&self, code: &str) -> Result<String, DiscordError> {
        let token: Value = self
            .http
            .post(format!("{}{}", self.api_base, Self::TOKEN_PATH))
            .form(&[
                ("client_id", self.client_id.as_str()),
                ("client_secret", self.client_secret.as_str()),
                ("grant_type", "authorization_code"),
                ("code", code),
                ("redirect_uri", self.redirect_uri.as_str()),
            ])
            .send()
            .await
            .map_err(|e| DiscordError::Http(e.to_string()))?
            .json()
            .await
            .map_err(|e| DiscordError::Http(e.to_string()))?;

        let access_token = token
            .get("access_token")
            .and_then(|t| t.as_str())
            .ok_or_else(|| DiscordError::Http("no access_token in the exchange".to_string()))?;

        let me: Value = self
            .http
            .get(format!("{}{}", self.api_base, Self::ME_PATH))
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| DiscordError::Http(e.to_string()))?
            .json()
            .await
            .map_err(|e| DiscordError::Http(e.to_string()))?;

        me.get("id")
            .and_then(|id| id.as_str())
            .map(String::from)
            .ok_or_else(|| DiscordError::Http("no id in the user payload".to_string()))
    }
}
